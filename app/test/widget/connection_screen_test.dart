import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/connection/connection_screen.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';

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
  });
}

/// A fake notifier that returns a preset [ServerConnectionState].
class _FakeNotifier extends ServerProfileNotifier {
  _FakeNotifier(this._initial);

  final ServerConnectionState _initial;

  @override
  Future<ServerConnectionState> build() async => _initial;
}
