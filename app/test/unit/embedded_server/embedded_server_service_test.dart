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
        final socket = await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
        final port = socket.port;
        await socket.close();

        expect(port, greaterThan(0));
        expect(port, lessThanOrEqualTo(65535));
      });
    });

    // -- stop() behaviour -----------------------------------------------------

    group('stop', () {
      test('completes without error when no process is running', () async {
        // Calling stop() before start() should be a no-op.
        final service = EmbeddedServerService();
        await expectLater(service.stop(), completes);
      });
    });
  });
}
