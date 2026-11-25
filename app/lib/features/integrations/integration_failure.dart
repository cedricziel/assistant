import 'package:dio/dio.dart';

/// Which kind of problem an integration request ran into.
enum IntegrationFailureKind { forbidden, conflict, notFound, invalid, other }

/// A failed integration request, described in words safe to show.
class IntegrationFailure {
  const IntegrationFailure(this.kind, this.message);

  final IntegrationFailureKind kind;
  final String message;
}

/// Turns any error from an integration request into a short user-facing
/// reason. [slug] names the slug being installed, so a `409` reads as a
/// duplicate rather than a stale write.
IntegrationFailure describeIntegrationFailure(Object error, {String? slug}) {
  if (error is! DioException) {
    return const IntegrationFailure(
      IntegrationFailureKind.other,
      'Something went wrong. Please try again.',
    );
  }
  final status = error.response?.statusCode;
  return switch (status) {
    null => const IntegrationFailure(
      IntegrationFailureKind.other,
      'The server could not be reached.',
    ),
    403 => const IntegrationFailure(
      IntegrationFailureKind.forbidden,
      'Your role or your organization\'s policy does not allow this.',
    ),
    404 => const IntegrationFailure(
      IntegrationFailureKind.notFound,
      'This integration no longer exists or is no longer visible to you.',
    ),
    409 => IntegrationFailure(
      IntegrationFailureKind.conflict,
      slug == null
          ? 'This integration was changed by someone else.'
          : 'An integration with the slug "$slug" already exists here. '
                'Choose a different slug.',
    ),
    422 => IntegrationFailure(
      IntegrationFailureKind.invalid,
      _serverMessage(error) ?? 'Some of the values are not valid.',
    ),
    _ => IntegrationFailure(
      IntegrationFailureKind.other,
      'The server answered with status $status.',
    ),
  };
}

/// The `{"error": "..."}` explanation the API attaches to 4xx responses.
String? _serverMessage(DioException error) {
  final data = error.response?.data;
  if (data is Map && data['error'] is String) return data['error'] as String;
  return null;
}
