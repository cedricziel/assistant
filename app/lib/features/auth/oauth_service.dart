import 'package:assistant_api/assistant_api.dart'
    show AssistantApi, ClientRegistrationSchema, OauthApi, TokenResponse;
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';

import 'oauth_credentials.dart';
import 'pkce.dart';

/// Handles the OAuth2 Authorization Code + PKCE flow against the assistant
/// server's OAuth2 endpoints.
///
/// Uses the generated [OauthApi] client for typed requests and response
/// parsing.  The only exception is [login], which needs custom redirect
/// handling that the generated client cannot express.
///
/// This service is stateless — it takes a server URL and performs HTTP calls
/// via [Dio]. Token storage is handled by the caller ([ContextRepository]).
class OAuthService {
  OAuthService({required this.serverUrl, Dio? dio}) {
    // When a pre-configured Dio is supplied (e.g. for tests with a mock
    // interceptor) we pass it through to AssistantApi so both the raw Dio
    // calls in login() and the generated OauthApi share the same instance.
    _api = AssistantApi(dio: dio, basePathOverride: serverUrl);
    _oauthApi = _api.getOauthApi();
  }

  final String serverUrl;
  late final AssistantApi _api;
  late final OauthApi _oauthApi;

  /// The underlying [Dio] instance (for the custom redirect flow in [login]).
  Dio get _dio => _api.dio;

  // -- Dynamic Client Registration (RFC 7591) --------------------------------

  /// Registers a new OAuth2 client via dynamic client registration.
  ///
  /// Returns the assigned [clientId]. The client is registered as a public
  /// client (no secret) with `authorization_code` grant type.
  Future<String> registerClient({
    required String clientName,
    required List<String> redirectUris,
  }) async {
    final response = await _oauthApi.register(
      clientRegistrationSchema: ClientRegistrationSchema(
        (b) => b
          ..clientName = clientName
          ..redirectUris = ListBuilder<String>(redirectUris)
          ..grantTypes = ListBuilder<String>([
            'authorization_code',
            'refresh_token',
          ])
          ..responseTypes = ListBuilder<String>(['code'])
          ..tokenEndpointAuthMethod = 'none',
      ),
    );

    return response.data!.clientId;
  }

  // -- Authorization Code + PKCE flow ----------------------------------------

  /// Builds the authorization URL for the browser redirect.
  ///
  /// The caller should open this URL in a browser. After the user
  /// authenticates, the server redirects to [redirectUri] with `?code=...`.
  Uri buildAuthorizationUrl({
    required String clientId,
    required String redirectUri,
    required PkceChallenge pkce,
    String? state,
    String? scope,
  }) {
    final params = <String, String>{
      'response_type': 'code',
      'client_id': clientId,
      'redirect_uri': redirectUri,
      'code_challenge': pkce.challenge,
      'code_challenge_method': 'S256',
    };
    if (state != null) params['state'] = state;
    if (scope != null) params['scope'] = scope;

    return Uri.parse(
      serverUrl,
    ).replace(path: '/oauth/authorize', queryParameters: params);
  }

  /// Exchanges an authorization code for tokens.
  ///
  /// The [code] comes from the redirect URI query parameter after the user
  /// approves. The [pkceVerifier] must match the challenge sent in the
  /// authorization request.
  Future<OAuthCredentials> exchangeCode({
    required String code,
    required String clientId,
    required String redirectUri,
    required String pkceVerifier,
  }) async {
    final response = await _oauthApi.token(
      grantType: 'authorization_code',
      code: code,
      clientId: clientId,
      redirectUri: redirectUri,
      codeVerifier: pkceVerifier,
    );

    return _toCredentials(response.data!, clientId);
  }

  // -- Direct login (email + password → auth code → tokens) ------------------

  /// Performs a direct login by posting credentials to the authorization
  /// endpoint and then exchanging the resulting auth code for tokens.
  ///
  /// This is a convenience for web/macOS where we can show an in-app login
  /// form rather than redirecting to a browser.
  ///
  /// Uses raw [Dio] because the generated client cannot express the
  /// `followRedirects: false` + Location-header capture flow.
  Future<OAuthCredentials> login({
    required String email,
    required String password,
    required String clientId,
    required String redirectUri,
    required PkceChallenge pkce,
  }) async {
    // POST credentials to get an auth code via redirect (302/303).
    //
    // The server redirects to /oauth/complete?code=... which returns JSON.
    // We handle three platform-dependent behaviours:
    //
    // 1. Dio honours `followRedirects: false` → 303 with Location header.
    // 2. Dio throws DioException on the 303 → extract Location from error.
    // 3. Dio follows the redirect (web/iOS) → response body is JSON from
    //    /oauth/complete containing `{"code": "..."}`.
    String? code;

    try {
      final authResponse = await _dio.post<dynamic>(
        '/oauth/authorize',
        data: {
          'client_id': clientId,
          'redirect_uri': redirectUri,
          'code_challenge': pkce.challenge,
          'email': email,
          'password': password,
        },
        options: Options(
          contentType: 'application/x-www-form-urlencoded',
          followRedirects: false,
          validateStatus: (status) =>
              status != null && (status >= 200 && status < 400),
        ),
      );

      // Path 1: Redirect captured — extract code from Location header.
      final location = authResponse.headers.value('location');
      if (location != null) {
        code = _extractCodeFromLocation(location, redirectUri);
      }

      // Path 3: Redirect was followed — extract code from response body.
      if (code == null && authResponse.data is Map) {
        code = (authResponse.data as Map)['code'] as String?;
      }
    } on DioException catch (e) {
      // Path 2: Dio threw on the redirect status.
      final status = e.response?.statusCode;
      if (status != null && status >= 300 && status < 400) {
        final location = e.response?.headers.value('location');
        if (location != null) {
          code = _extractCodeFromLocation(location, redirectUri);
        }
      }
      if (code == null) rethrow;
    }

    if (code == null) {
      throw OAuthException('No authorization code in server response');
    }

    // Exchange the auth code for tokens via the generated client.
    return exchangeCode(
      code: code,
      clientId: clientId,
      redirectUri: redirectUri,
      pkceVerifier: pkce.verifier,
    );
  }

  // -- Token refresh ---------------------------------------------------------

  /// Refreshes an expired access token using the stored refresh token.
  ///
  /// Returns new credentials with a fresh access token and potentially a
  /// rotated refresh token.
  Future<OAuthCredentials> refresh({
    required String refreshToken,
    required String clientId,
  }) async {
    final response = await _oauthApi.token(
      grantType: 'refresh_token',
      refreshToken: refreshToken,
      clientId: clientId,
    );

    return _toCredentials(response.data!, clientId);
  }

  // -- Token revocation ------------------------------------------------------

  /// Revokes a refresh token, invalidating the session.
  Future<void> revoke({required String token, String? clientId}) async {
    await _oauthApi.revoke(
      token: token,
      clientId: clientId,
      tokenTypeHint: 'refresh_token',
    );
  }

  // -- Server metadata -------------------------------------------------------

  /// Checks whether the server supports OAuth2 by fetching the well-known
  /// metadata endpoint. Returns `true` if the endpoint responds successfully.
  Future<bool> supportsOAuth() async {
    try {
      final response = await _oauthApi.metadata();
      return response.statusCode == 200;
    } catch (_) {
      return false;
    }
  }

  // -- Helpers ---------------------------------------------------------------

  /// Extracts the authorization code from a Location header value.
  ///
  /// Validates that the redirect target matches the registered [redirectUri]
  /// to prevent code theft via injected Location headers.
  String? _extractCodeFromLocation(String location, String redirectUri) {
    final locationUri = Uri.parse(location);
    final expectedUri = Uri.parse(redirectUri);
    if (locationUri.scheme != expectedUri.scheme ||
        locationUri.host != expectedUri.host ||
        locationUri.port != expectedUri.port ||
        locationUri.path != expectedUri.path) {
      throw OAuthException(
        'Redirect location does not match registered redirect_uri',
      );
    }

    final code = locationUri.queryParameters['code'];
    if (code == null) {
      final error = locationUri.queryParameters['error'] ?? 'unknown_error';
      final desc = locationUri.queryParameters['error_description'] ?? '';
      throw OAuthException('Authorization failed: $error $desc');
    }
    return code;
  }

  /// Converts a generated [TokenResponse] into our [OAuthCredentials].
  OAuthCredentials _toCredentials(TokenResponse token, String clientId) {
    if (token.refreshToken == null) {
      throw OAuthException('Server did not return a refresh token');
    }

    return OAuthCredentials(
      accessToken: token.accessToken,
      refreshToken: token.refreshToken!,
      expiresAt: DateTime.now().toUtc().add(Duration(seconds: token.expiresIn)),
      clientId: clientId,
    );
  }
}

/// Exception thrown when an OAuth2 operation fails.
class OAuthException implements Exception {
  const OAuthException(this.message);
  final String message;

  @override
  String toString() => 'OAuthException: $message';
}
