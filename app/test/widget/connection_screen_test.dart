import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/connection/connection_screen.dart';
import 'package:assistant_app/features/embedded_server/embedded_server_provider.dart';

void main() {
  group('ConnectionScreen', () {
    Widget buildUnderTest({ServerConnectionState? initialState}) {
      return ProviderScope(
        overrides: [
          if (initialState != null)
            serverProfileProvider.overrideWith(
              () => _FakeNotifier(initialState),
            ),
        ],
        child: const MaterialApp(
          home: ConnectionScreen(),
        ),
      );
    }

    testWidgets('shows token field on all platforms', (tester) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pump();

      expect(find.byKey(const Key('token_field')), findsOneWidget);
      expect(find.byKey(const Key('connect_button')), findsOneWidget);
    });

    testWidgets('shows "Invalid token" error when state has that error',
        (tester) async {
      await tester.pumpWidget(
        buildUnderTest(
          initialState: const ServerConnectionState(
            error: 'Invalid token',
          ),
        ),
      );
      await tester.pump();

      expect(find.byKey(const Key('error_message')), findsOneWidget);
      expect(find.text('Invalid token'), findsOneWidget);
    });

    testWidgets('shows "Server unreachable" error when state has that error',
        (tester) async {
      await tester.pumpWidget(
        buildUnderTest(
          initialState: const ServerConnectionState(
            error: 'Server unreachable',
          ),
        ),
      );
      await tester.pump();

      expect(find.byKey(const Key('error_message')), findsOneWidget);
      expect(find.text('Server unreachable'), findsOneWidget);
    });

    testWidgets('connect button is disabled while connecting', (tester) async {
      await tester.pumpWidget(
        buildUnderTest(
          initialState: const ServerConnectionState(isConnecting: true),
        ),
      );
      await tester.pump();

      final button = tester.widget<FilledButton>(
        find.byKey(const Key('connect_button')),
      );
      expect(button.onPressed, isNull);
    });

    testWidgets('validates empty token field', (tester) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pump();

      // Tap connect with empty token.
      await tester.tap(find.byKey(const Key('connect_button')));
      await tester.pump();

      expect(find.text('Token is required'), findsOneWidget);
    });

    // -- Embedded mode --------------------------------------------------------

    testWidgets(
        'embedded mode toggle is NOT shown in test environment '
        '(binary not present in test runner bundle)', (tester) async {
      // In `flutter test`, EmbeddedServerService.isAvailable is always false
      // because the binary is never placed in the test sandbox.
      // Verify the toggle is hidden and remote fields are shown by default.
      await tester.pumpWidget(buildUnderTest());
      await tester.pump();

      expect(
        find.byKey(const Key('server_mode_toggle')),
        findsNothing,
        reason: 'Toggle should only appear when the bundled binary is present',
      );

      // Remote fields are shown when embedded mode is unavailable.
      expect(find.byKey(const Key('server_url_field')), findsOneWidget);
      expect(find.byKey(const Key('token_field')), findsOneWidget);
    });

    testWidgets('embedded provider override wires correctly', (tester) async {
      // Override the embedded-server provider to return a known starting state.
      // Since EmbeddedServerService.isAvailable is false in the test sandbox
      // (no bundled binary), the toggle won't render — but we verify the
      // provider override compiles and the screen still renders without errors.
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            embeddedServerProvider.overrideWith(
              () => _FakeEmbeddedNotifier(const EmbeddedServerStarting()),
            ),
          ],
          child: const MaterialApp(home: ConnectionScreen()),
        ),
      );
      await tester.pump();

      // Screen renders without error; toggle absent because isAvailable=false.
      expect(find.byKey(const Key('server_mode_toggle')), findsNothing);
      expect(find.byType(ConnectionScreen), findsOneWidget);
    });
  });
}

/// A fake notifier that returns a preset [ServerConnectionState].
class _FakeNotifier extends ServerProfileNotifier {
  _FakeNotifier(this._initial);

  final ServerConnectionState _initial;

  @override
  Future<ServerConnectionState> build() async => _initial;
}

/// A fake embedded-server notifier that returns a preset state.
class _FakeEmbeddedNotifier extends EmbeddedServerNotifier {
  _FakeEmbeddedNotifier(this._initial);

  final EmbeddedServerState _initial;

  @override
  Future<EmbeddedServerState> build() async => _initial;
}
