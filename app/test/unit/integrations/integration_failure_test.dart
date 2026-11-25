import 'package:assistant_app/features/integrations/integration_failure.dart';
import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';

DioException _status(int status, {Object? body}) => DioException(
  requestOptions: RequestOptions(path: '/api/integrations'),
  response: Response(
    requestOptions: RequestOptions(path: '/api/integrations'),
    statusCode: status,
    data: body,
  ),
  type: DioExceptionType.badResponse,
);

void main() {
  group('describeIntegrationFailure', () {
    test('maps 403 to a policy message', () {
      final failure = describeIntegrationFailure(_status(403));
      expect(failure.kind, IntegrationFailureKind.forbidden);
      expect(failure.message, contains('does not allow'));
    });

    test('maps 409 on an install to a duplicate-slug message', () {
      final failure = describeIntegrationFailure(_status(409), slug: 'linear');
      expect(failure.kind, IntegrationFailureKind.conflict);
      expect(failure.message, contains('"linear"'));
      expect(failure.message, contains('already'));
    });

    test('maps 409 without a slug to a stale-data message', () {
      final failure = describeIntegrationFailure(_status(409));
      expect(failure.kind, IntegrationFailureKind.conflict);
      expect(failure.message, contains('changed'));
    });

    test('maps 404 to a gone message', () {
      final failure = describeIntegrationFailure(_status(404));
      expect(failure.kind, IntegrationFailureKind.notFound);
    });

    test('maps 422 to the server explanation', () {
      final failure = describeIntegrationFailure(
        _status(422, body: {'error': 'slug must be kebab-case'}),
      );
      expect(failure.kind, IntegrationFailureKind.invalid);
      expect(failure.message, contains('slug must be kebab-case'));
    });

    test('maps 422 without a readable body to a generic message', () {
      final failure = describeIntegrationFailure(_status(422, body: 'nope'));
      expect(failure.kind, IntegrationFailureKind.invalid);
      expect(failure.message, isNotEmpty);
    });

    test('maps a missing response to an unreachable message', () {
      final failure = describeIntegrationFailure(
        DioException(
          requestOptions: RequestOptions(path: '/x'),
          type: DioExceptionType.connectionError,
        ),
      );
      expect(failure.kind, IntegrationFailureKind.other);
      expect(failure.message, contains('could not be reached'));
    });

    test('maps other statuses and errors without leaking internals', () {
      expect(
        describeIntegrationFailure(_status(500)).message,
        contains('status 500'),
      );
      expect(
        describeIntegrationFailure(StateError('secret internals')).message,
        isNot(contains('secret')),
      );
    });
  });
}
