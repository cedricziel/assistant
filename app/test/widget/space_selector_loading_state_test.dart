// Asserts the selector distinguishes "API client not yet ready" from
// "API returned empty" — while the connection (serverProfileProvider) is
// still resolving, the screen MUST show a spinner, not the empty-state card.
//
// Spec: openspec/specs/space-selector-resilience/spec.md
//   "Selector providers distinguish 'API not ready' from 'API returned empty'"

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/spaces/space_selector_screen.dart';

class _PendingServerProfileNotifier extends ServerProfileNotifier {
  _PendingServerProfileNotifier(this._gate);
  final Completer<ServerConnectionState> _gate;
  @override
  Future<ServerConnectionState> build() => _gate.future;
}

class _SettledNoProfileNotifier extends ServerProfileNotifier {
  @override
  Future<ServerConnectionState> build() async => const ServerConnectionState();
}

void main() {
  testWidgets(
    'OrgList shows a spinner (NOT "No organizations found") while serverProfileProvider is loading',
    (tester) async {
      final gate = Completer<ServerConnectionState>();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            // The transient state we're testing: connection still resolving,
            // apiClient null. The real OrgsNotifier MUST stay in loading,
            // not short-circuit to AsyncData([]).
            serverProfileProvider.overrideWith(
              () => _PendingServerProfileNotifier(gate),
            ),
          ],
          child: const MaterialApp(home: SpaceSelectorScreen()),
        ),
      );
      await tester.pump();

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(find.textContaining('No organizations found'), findsNothing);
      expect(find.textContaining('Failed to load'), findsNothing);

      // Don't leave a dangling future.
      gate.complete(const ServerConnectionState());
      await tester.pumpAndSettle();
    },
  );

  testWidgets(
    'OrgList renders empty card when serverProfileProvider settles with no profile',
    (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            serverProfileProvider.overrideWith(
              () => _SettledNoProfileNotifier(),
            ),
          ],
          child: const MaterialApp(home: SpaceSelectorScreen()),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.textContaining('No organizations found'), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsNothing);
    },
  );
}
