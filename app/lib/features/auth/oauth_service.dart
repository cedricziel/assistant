import 'package:assistant_api/assistant_api.dart'
    show AssistantApi, ClientRegistrationSchema, OauthApi, TokenResponse;
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';

import 'oauth_credentials.dart';
import 'pkce.dart';

/// Handles the OAuth2 Authorization Code + PKCE flow against the assistant
/// server's OAuth2 endpoints.
///
/// Uses the generated [OauthApi] client for typed requests and response
/// parsing.
///
/// This service is stateless — it takes a server URL and performs HTTP calls
/// via [Dio]. Token storage is handled by the caller ([ContextRepository]).
class OAuthService {
  OAuthService({required this.serverUrl, Dio? dio}) {
    // Tests pass a Dio with a mock interceptor. Without one, the generated
    // client's own default gives up after 3 seconds.
    _api = AssistantApi(
      dio:
          dio ??
          Dio(
            BaseOptions(
              connectTimeout: const Duration(seconds: 15),
              receiveTimeout: const Duration(seconds: 60),
            ),
          ),
      basePathOverride: serverUrl,
    );
    _oauthApi = _api.getOauthApi();
  }

  final String serverUrl;
  late final AssistantApi _api;
  late final OauthApi _oauthApi;

  @visibleForTesting
  Dio get dio => _api.dio;

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

    return Uri.parse(serverUrl)
        .replace(path: '/oauth/authorize', queryParameters: params);
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
