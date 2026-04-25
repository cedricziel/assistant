import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/auth/oauth_service.dart';
import 'package:assistant_app/features/auth/pkce.dart';

/// A Dio interceptor that captures requests and returns canned responses.
class MockInterceptor extends Interceptor {
  final List<RequestOptions> requests = [];
  final Map<String, MockResponse> _responses = {};

  void when(String path, {required MockResponse respond}) {
    _responses[path] = respond;
  }

  @override
  void onRequest(RequestOptions options, RequestInterceptorHandler handler) {
    requests.add(options);
    final key = options.path;
    final mock = _responses[key];
    if (mock != null) {
      if (mock.error != null) {
        handler.reject(
          DioException(
            requestOptions: options,
            response: Response(
              requestOptions: options,
              statusCode: mock.statusCode,
              data: mock.data,
            ),
            type: DioExceptionType.badResponse,
          ),
        );
      } else {
        handler.resolve(
          Response(
            requestOptions: options,
            statusCode: mock.statusCode,
            data: mock.data,
            headers: mock.headers != null
                ? Headers.fromMap(mock.headers!.map((k, v) => MapEntry(k, [v])))
                : Headers(),
          ),
        );
      }
    } else {
      handler.reject(
        DioException(
          requestOptions: options,
          type: DioExceptionType.connectionError,
          error: 'No mock configured for $key',
        ),
      );
    }
  }
}

class MockResponse {
  const MockResponse({
    this.statusCode = 200,
    this.data,
    this.headers,
    this.error,
  });

  final int statusCode;
  final dynamic data;
  final Map<String, String>? headers;
  final String? error;
}

void main() {
  late Dio dio;
  late MockInterceptor mock;
  late OAuthService service;

  setUp(() {
    dio = Dio(BaseOptions(baseUrl: 'http://localhost:8080'));
    mock = MockInterceptor();
    dio.interceptors.add(mock);
    service = OAuthService(serverUrl: 'http://localhost:8080', dio: dio);
  });

  group('OAuthService', () {
    group('registerClient', () {
      test('posts registration and returns client_id', () async {
        mock.when(
          '/oauth/register',
          respond: const MockResponse(
            statusCode: 200,
            data: {
              'client_id': 'new_client_123',
              'client_name': 'Test App',
              'grant_types': ['authorization_code', 'refresh_token'],
              'redirect_uris': ['http://localhost:4040/callback'],
              'token_endpoint_auth_method': 'none',
            },
          ),
        );

        final clientId = await service.registerClient(
          clientName: 'Test App',
          redirectUris: ['http://localhost:4040/callback'],
        );

        expect(clientId, 'new_client_123');
        expect(mock.requests, hasLength(1));
        expect(mock.requests.first.path, '/oauth/register');
        expect(mock.requests.first.method, 'POST');
      });

      test('throws on server error', () async {
        mock.when(
          '/oauth/register',
          respond: const MockResponse(
            statusCode: 400,
            data: {'error': 'invalid_request'},
            error: 'bad request',
          ),
        );

        expect(
          () => service.registerClient(
            clientName: 'Bad',
            redirectUris: ['http://localhost/cb'],
          ),
          throwsA(isA<DioException>()),
        );
      });
    });

    group('buildAuthorizationUrl', () {
      test('constructs correct URL with all parameters', () {
        final pkce = PkceChallenge.generate();
        final url = service.buildAuthorizationUrl(
          clientId: 'client_1',
          redirectUri: 'http://localhost:4040/callback',
          pkce: pkce,
          state: 'random_state',
          scope: 'personas:read conversations:write',
        );

        expect(url.scheme, 'http');
        expect(url.host, 'localhost');
        expect(url.port, 8080);
        expect(url.path, '/oauth/authorize');
        expect(url.queryParameters['response_type'], 'code');
        expect(url.queryParameters['client_id'], 'client_1');
        expect(
          url.queryParameters['redirect_uri'],
          'http://localhost:4040/callback',
        );
        expect(url.queryParameters['code_challenge'], pkce.challenge);
        expect(url.queryParameters['code_challenge_method'], 'S256');
        expect(url.queryParameters['state'], 'random_state');
        expect(
          url.queryParameters['scope'],
          'personas:read conversations:write',
        );
      });

      test('omits optional parameters when null', () {
        final pkce = PkceChallenge.generate();
        final url = service.buildAuthorizationUrl(
          clientId: 'client_1',
          redirectUri: 'http://localhost/cb',
          pkce: pkce,
        );

        expect(url.queryParameters, isNot(contains('state')));
        expect(url.queryParameters, isNot(contains('scope')));
      });
    });

    group('exchangeCode', () {
      test('exchanges auth code for credentials', () async {
        mock.when(
          '/oauth/token',
          respond: const MockResponse(
            statusCode: 200,
            data: {
              'access_token': 'jwt_access_token',
              'refresh_token': 'rt_refresh_token',
              'token_type': 'Bearer',
              'expires_in': 3600,
            },
          ),
        );

        final creds = await service.exchangeCode(
          code: 'auth_code_123',
          clientId: 'client_1',
          redirectUri: 'http://localhost/cb',
          pkceVerifier: 'verifier_abc',
        );

        expect(creds.accessToken, 'jwt_access_token');
        expect(creds.refreshToken, 'rt_refresh_token');
        expect(creds.clientId, 'client_1');
        expect(creds.expiresAt.isAfter(DateTime.now().toUtc()), isTrue);

        final req = mock.requests.first;
        expect(req.path, '/oauth/token');
        expect(req.data, containsPair('grant_type', 'authorization_code'));
        expect(req.data, containsPair('code', 'auth_code_123'));
        expect(req.data, containsPair('code_verifier', 'verifier_abc'));
      });

      test('throws OAuthException when no refresh token returned', () async {
        mock.when(
          '/oauth/token',
          respond: const MockResponse(
            statusCode: 200,
            data: {
              'access_token': 'jwt',
              'token_type': 'Bearer',
              'expires_in': 3600,
            },
          ),
        );

        expect(
          () => service.exchangeCode(
            code: 'code',
            clientId: 'c',
            redirectUri: 'http://localhost/cb',
            pkceVerifier: 'v',
          ),
          throwsA(isA<OAuthException>()),
        );
      });
    });

    group('login', () {
      test('posts credentials and exchanges code for tokens', () async {
        // Step 1: POST /oauth/authorize returns redirect with code
        mock.when(
          '/oauth/authorize',
          respond: const MockResponse(
            statusCode: 302,
            data: null,
            headers: {'location': 'http://localhost/cb?code=code_xyz&state=s'},
          ),
        );

        // Step 2: POST /oauth/token returns tokens
        mock.when(
          '/oauth/token',
          respond: const MockResponse(
            statusCode: 200,
            data: {
              'access_token': 'at_login',
              'refresh_token': 'rt_login',
              'token_type': 'Bearer',
              'expires_in': 3600,
            },
          ),
        );

        final pkce = PkceChallenge.generate();
        final creds = await service.login(
          email: 'alice@example.com',
          password: 'secret',
          clientId: 'client_1',
          redirectUri: 'http://localhost/cb',
          pkce: pkce,
        );

        expect(creds.accessToken, 'at_login');
        expect(creds.refreshToken, 'rt_login');
        expect(creds.clientId, 'client_1');

        // Verify auth request included credentials
        final authReq = mock.requests.firstWhere(
          (r) => r.path == '/oauth/authorize',
        );
        expect(authReq.data, containsPair('email', 'alice@example.com'));
        expect(authReq.data, containsPair('password', 'secret'));
        expect(authReq.data, containsPair('code_challenge', pkce.challenge));

        // Verify token request used correct code
        final tokenReq = mock.requests.firstWhere(
          (r) => r.path == '/oauth/token',
        );
        expect(tokenReq.data, containsPair('code', 'code_xyz'));
        expect(tokenReq.data, containsPair('code_verifier', pkce.verifier));
      });

      test('throws OAuthException when redirect has no code', () async {
        mock.when(
          '/oauth/authorize',
          respond: const MockResponse(
            statusCode: 302,
            headers: {
              'location':
                  'http://localhost/cb?error=access_denied&error_description=Bad+creds',
            },
          ),
        );

        final pkce = PkceChallenge.generate();
        expect(
          () => service.login(
            email: 'bad@example.com',
            password: 'wrong',
            clientId: 'c',
            redirectUri: 'http://localhost/cb',
            pkce: pkce,
          ),
          throwsA(
            isA<OAuthException>().having(
              (e) => e.message,
              'message',
              contains('access_denied'),
            ),
          ),
        );
      });

      test('throws OAuthException when no Location header', () async {
        mock.when(
          '/oauth/authorize',
          respond: const MockResponse(statusCode: 200, data: 'OK'),
        );

        final pkce = PkceChallenge.generate();
        expect(
          () => service.login(
            email: 'a@b.com',
            password: 'p',
            clientId: 'c',
            redirectUri: 'http://localhost/cb',
            pkce: pkce,
          ),
          throwsA(isA<OAuthException>()),
        );
      });
    });

    group('refresh', () {
      test('exchanges refresh token for new credentials', () async {
        mock.when(
          '/oauth/token',
          respond: const MockResponse(
            statusCode: 200,
            data: {
              'access_token': 'at_refreshed',
              'refresh_token': 'rt_rotated',
              'token_type': 'Bearer',
              'expires_in': 3600,
            },
          ),
        );

        final creds = await service.refresh(
          refreshToken: 'rt_old',
          clientId: 'client_1',
        );

        expect(creds.accessToken, 'at_refreshed');
        expect(creds.refreshToken, 'rt_rotated');

        final req = mock.requests.first;
        expect(req.data, containsPair('grant_type', 'refresh_token'));
        expect(req.data, containsPair('refresh_token', 'rt_old'));
      });

      test('throws DioException on 401 (revoked token)', () async {
        mock.when(
          '/oauth/token',
          respond: const MockResponse(
            statusCode: 401,
            data: {'error': 'invalid_grant'},
            error: 'unauthorized',
          ),
        );

        expect(
          () => service.refresh(refreshToken: 'bad', clientId: 'c'),
          throwsA(isA<DioException>()),
        );
      });
    });

    group('revoke', () {
      test('posts revocation request', () async {
        mock.when(
          '/oauth/revoke',
          respond: const MockResponse(statusCode: 200),
        );

        await service.revoke(token: 'rt_to_revoke', clientId: 'client_1');

        expect(mock.requests, hasLength(1));
        final req = mock.requests.first;
        expect(req.path, '/oauth/revoke');
        expect(req.data, containsPair('token', 'rt_to_revoke'));
        expect(req.data, containsPair('token_type_hint', 'refresh_token'));
      });
    });

    group('supportsOAuth', () {
      test('returns true when metadata endpoint responds 200', () async {
        mock.when(
          '/.well-known/oauth-authorization-server',
          respond: const MockResponse(
            statusCode: 200,
            data: {'issuer': 'http://localhost:8080'},
          ),
        );

        expect(await service.supportsOAuth(), isTrue);
      });

      test('returns false when metadata endpoint fails', () async {
        mock.when(
          '/.well-known/oauth-authorization-server',
          respond: const MockResponse(statusCode: 404, error: 'not found'),
        );

        expect(await service.supportsOAuth(), isFalse);
      });
    });
  });
}
