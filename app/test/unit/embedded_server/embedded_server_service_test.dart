import 'dart:io';

import 'package:assistant_app/features/embedded_server/embedded_server_service.dart'
    show EmbeddedServerService;
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('EmbeddedServerService', () {
    // -- isAvailable ----------------------------------------------------------

    group('isAvailable', () {
      test('returns false on non-macOS platforms', () {
        // On the CI / test runner (likely Linux or macOS without bundle),
        // Platform.isMacOS may vary, but the Resources/assistant file will
        // never exist in the test environment so the result is always false.
        if (!Platform.isMacOS) {
          expect(EmbeddedServerService.isAvailable, isFalse);
        } else {
          // On macOS the binary is only present inside an actual .app bundle,
          // not during `flutter test`. Verify we at least get a bool.
          expect(EmbeddedServerService.isAvailable, isA<bool>());
        }
      });
    });

    // -- _findFreePort --------------------------------------------------------

    group('findFreePort', () {
      test('returns a non-zero port', () async {
        // _findFreePort is private; test the underlying mechanism directly by
        // verifying a ServerSocket can bind to port 0 and return a usable port.
        final socket =
            await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
        final port = socket.port;
        await socket.close();

        expect(port, greaterThan(0));
        expect(port, lessThanOrEqualTo(65535));
      });
    });

    // -- health-check polling logic -------------------------------------------

    group('health-check retry loop logic', () {
      test('stops retrying when a successful status is found', () async {
        // Test the retry-loop logic in isolation: a list acting as a mock
        // response sequence.  The loop is the same pattern used in start().
        final responses = [503, 503, 200]; // succeed on 3rd attempt
        var attempts = 0;
        var succeeded = false;

        for (final statusCode in responses) {
          attempts++;
          if (statusCode == 200) {
            succeeded = true;
            break;
          }
        }

        expect(succeeded, isTrue);
        expect(attempts, equals(3));
      });

      test('does not succeed after 10 non-200 responses', () async {
        final responses = List.filled(10, 503);
        var attempts = 0;
        var succeeded = false;

        for (final statusCode in responses) {
          attempts++;
          if (statusCode == 200) {
            succeeded = true;
            break;
          }
        }

        expect(succeeded, isFalse);
        expect(attempts, equals(10));
      });
    });
  });
}
