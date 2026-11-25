import 'dart:async';

import 'package:dio/dio.dart';

/// Returns `true` if [error] looks like a transient connection failure
/// (iOS backgrounding, network blip) rather than a permanent server error.
bool isTransientStreamError(Object error) {
  if (error is TimeoutException) return true;
  if (error is DioException && error.type == DioExceptionType.connectionError) {
    return true;
  }
  final msg = error.toString();
  return msg.contains('HttpException') ||
      msg.contains('Connection closed') ||
      msg.contains('Connection reset') ||
      msg.contains('SocketException');
}

/// Returns a user-friendly error message for stream exceptions.
String friendlyStreamErrorMessage(Object error) {
  if (isTransientStreamError(error)) {
    return 'Connection lost — tap retry to resend';
  }
  return 'Stream error: $error';
}
