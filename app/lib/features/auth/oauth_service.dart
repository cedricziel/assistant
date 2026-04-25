import 'package:dio/dio.dart';

import 'oauth_credentials.dart';
import 'pkce.dart';

/// Handles the OAuth2 Authorization Code + PKCE flow against the assistant
/// server's OAuth2 endpoints.
///
/// This service is stateless — it takes a server URL and performs HTTP calls
/// via [Dio]. Token storage is handled by the caller ([ContextRepository]).
class OAuthService {
  OAuthService({required this.serverUrl, Dio? dio})
    : _dio = dio ?? Dio(BaseOptions(baseUrl: serverUrl));

  final String serverUrl;
  final Dio _dio;

  // -- Dynamic Client Registration (RFC 7591) --------------------------------

  /// Registers a new OAuth2 client via dynamic client registration.
  ///
  /// Returns the assigned [clientId]. The client is registered as a public
  /// client (no secret) with `authorization_code` grant type.
  Future<String> registerClient({
    required String clientName,
    required List<String> redirectUris,
  }) async {
    final response = await _dio.post<Map<String, dynamic>>(
      '/oauth/register',
      data: {
        'client_name': clientName,
        'redirect_uris': redirectUris,
        'grant_types': ['authorization_code', 'refresh_token'],
        'response_types': ['code'],
        'token_endpoint_auth_method': 'none',
      },
      options: Options(contentType: 'application/json'),
    );

    final data = response.data!;
    return data['client_id'] as String;
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
    final response = await _dio.post<Map<String, dynamic>>(
      '/oauth/token',
      data: {
        'grant_type': 'authorization_code',
        'code': code,
        'client_id': clientId,
        'redirect_uri': redirectUri,
        'code_verifier': pkceVerifier,
      },
      options: Options(contentType: 'application/x-www-form-urlencoded'),
    );

    return _parseTokenResponse(response.data!, clientId);
  }

  // -- Direct login (email + password → auth code → tokens) ------------------

  /// Performs a direct login by posting credentials to the authorization
  /// endpoint and then exchanging the resulting auth code for tokens.
  ///
  /// This is a convenience for web/macOS where we can show an in-app login
  /// form rather than redirecting to a browser.
  Future<OAuthCredentials> login({
    required String email,
    required String password,
    required String clientId,
    required String redirectUri,
    required PkceChallenge pkce,
  }) async {
    // POST credentials to get an auth code via redirect.
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

    // Extract the auth code from the redirect Location header.
    final location = authResponse.headers.value('location');
    if (location == null) {
      throw OAuthException('No redirect location in authorization response');
    }

    final locationUri = Uri.parse(location);
    final code = locationUri.queryParameters['code'];
    if (code == null) {
      final error = locationUri.queryParameters['error'] ?? 'unknown_error';
      final desc = locationUri.queryParameters['error_description'] ?? '';
      throw OAuthException('Authorization failed: $error $desc');
    }

    // Exchange the auth code for tokens.
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
    final response = await _dio.post<Map<String, dynamic>>(
      '/oauth/token',
      data: {
        'grant_type': 'refresh_token',
        'refresh_token': refreshToken,
        'client_id': clientId,
      },
      options: Options(contentType: 'application/x-www-form-urlencoded'),
    );

    return _parseTokenResponse(response.data!, clientId);
  }

  // -- Token revocation ------------------------------------------------------

  /// Revokes a refresh token, invalidating the session.
  Future<void> revoke({required String token, String? clientId}) async {
    await _dio.post<void>(
      '/oauth/revoke',
      data: {
        'token': token,
        'client_id': ?clientId,
        'token_type_hint': 'refresh_token',
      },
      options: Options(contentType: 'application/x-www-form-urlencoded'),
    );
  }

  // -- Server metadata -------------------------------------------------------

  /// Checks whether the server supports OAuth2 by fetching the well-known
  /// metadata endpoint. Returns `true` if the endpoint responds successfully.
  Future<bool> supportsOAuth() async {
    try {
      final response = await _dio.get<dynamic>(
        '/.well-known/oauth-authorization-server',
      );
      return response.statusCode == 200;
    } catch (_) {
      return false;
    }
  }

  // -- Helpers ---------------------------------------------------------------

  OAuthCredentials _parseTokenResponse(
    Map<String, dynamic> data,
    String clientId,
  ) {
    final accessToken = data['access_token'] as String;
    final refreshToken = data['refresh_token'] as String?;
    final expiresIn = data['expires_in'] as int;

    if (refreshToken == null) {
      throw OAuthException('Server did not return a refresh token');
    }

    return OAuthCredentials(
      accessToken: accessToken,
      refreshToken: refreshToken,
      expiresAt: DateTime.now().toUtc().add(Duration(seconds: expiresIn)),
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
