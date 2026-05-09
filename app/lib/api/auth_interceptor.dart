import 'dart:async';

import 'package:dio/dio.dart';

/// Returns the new bearer token on success, or `null` if refresh failed.
typedef RefreshTokensCallback = Future<String?> Function();

/// Called when the session is unrecoverable (no refresh token, refresh
/// rejected, or a second 401 after retry). The wiring layer is expected to
/// deactivate the active context so the router redirects to `/login`.
typedef OnAuthExpiredCallback = Future<void> Function();

/// Dio interceptor that recovers from 401 responses by attempting a single
/// OAuth2 refresh and retrying the request once. If refresh fails — or the
/// retry itself returns 401 — the interceptor invokes [onAuthExpired] and
/// propagates the original 401 to the caller.
///
/// Concurrent 401s are coalesced onto a single refresh future
/// (single-flight) to avoid stampeding the token endpoint.
class AuthRecoveryInterceptor extends Interceptor {
  AuthRecoveryInterceptor({
    required this.refreshTokens,
    required this.onAuthExpired,
    Dio? dio,
  }) : _retryDio = dio;

  final RefreshTokensCallback refreshTokens;
  final OnAuthExpiredCallback onAuthExpired;

  /// Dio used to dispatch the retry. When null, the interceptor lazily
  /// constructs a fresh Dio per retry — sufficient for the unit test, but
  /// production wiring (`apiClientProvider`) MUST pass the real Dio so
  /// adapter overrides and other interceptors still apply on the retry.
  Dio? _retryDio;

  // ignore: use_setters_to_change_properties
  /// Used by [ApiClient] to attach the owning Dio after the interceptor is
  /// constructed (we can't pass it via constructor without a circular ref).
  void attachRetryDio(Dio dio) {
    _retryDio = dio;
  }

  /// In-flight refresh shared by all concurrent 401 handlers.
  Completer<String?>? _inFlight;

  /// Marker placed in `RequestOptions.extra` to prevent retry loops.
  static const _retriedKey = 'x-retried';

  @override
  Future<void> onError(
    DioException err,
    ErrorInterceptorHandler handler,
  ) async {
    if (err.response?.statusCode != 401) {
      handler.next(err);
      return;
    }
    if (err.requestOptions.extra[_retriedKey] == true) {
      // Already retried once — give up and let the router handle the redirect.
      await onAuthExpired();
      handler.next(err);
      return;
    }

    final newToken = await _refreshOnce();
    if (newToken == null) {
      await onAuthExpired();
      handler.next(err);
      return;
    }

    // Retry with the new bearer and a marker so a second 401 falls through.
    final retryOptions = Options(
      method: err.requestOptions.method,
      headers: {
        ...err.requestOptions.headers,
        'Authorization': 'Bearer $newToken',
      },
      responseType: err.requestOptions.responseType,
      contentType: err.requestOptions.contentType,
      sendTimeout: err.requestOptions.sendTimeout,
      receiveTimeout: err.requestOptions.receiveTimeout,
      followRedirects: err.requestOptions.followRedirects,
      validateStatus: err.requestOptions.validateStatus,
      receiveDataWhenStatusError: err.requestOptions.receiveDataWhenStatusError,
      extra: {...err.requestOptions.extra, _retriedKey: true},
    );

    final dio =
        _retryDio ?? Dio(BaseOptions(baseUrl: err.requestOptions.baseUrl));
    try {
      final response = await dio.request<dynamic>(
        err.requestOptions.path,
        data: err.requestOptions.data,
        queryParameters: err.requestOptions.queryParameters,
        cancelToken: err.requestOptions.cancelToken,
        options: retryOptions,
      );
      handler.resolve(response);
    } on DioException catch (retryErr) {
      // The retry runs through this same interceptor chain. If it returns 401,
      // the inner onError invocation (with `extra[_retriedKey] == true`) has
      // already called [onAuthExpired]; do NOT call it again here. For all
      // other failure modes, just propagate.
      handler.next(retryErr);
    }
  }

  Future<String?> _refreshOnce() async {
    final existing = _inFlight;
    if (existing != null) {
      return existing.future;
    }
    final completer = Completer<String?>();
    _inFlight = completer;
    try {
      final token = await refreshTokens();
      completer.complete(token);
      return token;
    } catch (_) {
      completer.complete(null);
      return null;
    } finally {
      _inFlight = null;
    }
  }
}
