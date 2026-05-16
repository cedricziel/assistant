import 'package:dio/dio.dart';

/// Stub used on web builds. Conditional import in `api_client.dart` selects
/// this when `dart.library.io` is unavailable. Web routes through Dio's
/// `BrowserHttpClientAdapter`, which already streams `fetch` responses
/// correctly — no native adapter is needed.
void installNativeHttpAdapter(Dio dio) {
  // No-op on web.
}
