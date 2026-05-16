import 'dart:io' show Platform;

import 'package:dio/dio.dart';
import 'package:native_dio_adapter/native_dio_adapter.dart';

/// Install the platform-native HTTP stack on iOS / macOS (NSURLSession via
/// cupertino_http) and Android (Cronet). Selected on non-web builds via
/// conditional import from `api_client.dart`.
///
/// dart:io's HttpClient — Dio's default transport — buffers SSE bodies on
/// iOS / iPadOS until the connection closes, which causes the "jumpy dots
/// forever" behaviour. NativeAdapter delegates to platform HTTP stacks that
/// stream chunked transfers in real time.
///
/// Skipped under `flutter test`: NativeAdapter eagerly instantiates a native
/// URLSession via cupertino_http, which requires the iOS / macOS platform
/// binding that unit tests don't provide. Detected via the `FLUTTER_TEST`
/// env var the test runner sets.
void installNativeHttpAdapter(Dio dio) {
  if (Platform.environment.containsKey('FLUTTER_TEST')) {
    return;
  }
  dio.httpClientAdapter = NativeAdapter();
}
