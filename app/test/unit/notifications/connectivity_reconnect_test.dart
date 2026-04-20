import 'dart:async';

import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/connectivity_provider.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/notifications/agent_event_listener.dart';

/// Tracks calls to attemptReconnect.
class _MockChatNotifier extends AsyncNotifier<ChatState>
    implements ChatNotifier {
  int reconnectCallCount = 0;

  @override
  Future<ChatState> build() async => const ChatState();

  @override
  Future<void> attemptReconnect() async {
    reconnectCallCount++;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => null;
}

void main() {
  group('AgentEventListener connectivity reconnect', () {
    testWidgets('calls attemptReconnect on none→wifi transition', (
      tester,
    ) async {
      final connectivityController =
          StreamController<List<ConnectivityResult>>();
      addTearDown(connectivityController.close);

      final mockChat = _MockChatNotifier();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            connectivityProvider.overrideWith(
              (ref) => connectivityController.stream,
            ),
            chatProvider.overrideWith(() => mockChat),
          ],
          child: const MaterialApp(
            home: AgentEventListener(child: SizedBox.shrink()),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Emit offline state.
      connectivityController.add([ConnectivityResult.none]);
      await tester.pumpAndSettle();

      // Emit online state — should trigger reconnect.
      connectivityController.add([ConnectivityResult.wifi]);
      await tester.pumpAndSettle();

      expect(
        mockChat.reconnectCallCount,
        1,
        reason: 'attemptReconnect should be called once on none→wifi',
      );
    });

    testWidgets('does not call attemptReconnect on wifi→wifi transition', (
      tester,
    ) async {
      final connectivityController =
          StreamController<List<ConnectivityResult>>();
      addTearDown(connectivityController.close);

      final mockChat = _MockChatNotifier();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            connectivityProvider.overrideWith(
              (ref) => connectivityController.stream,
            ),
            chatProvider.overrideWith(() => mockChat),
          ],
          child: const MaterialApp(
            home: AgentEventListener(child: SizedBox.shrink()),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Emit online state.
      connectivityController.add([ConnectivityResult.wifi]);
      await tester.pumpAndSettle();

      // Emit online state again.
      connectivityController.add([ConnectivityResult.wifi]);
      await tester.pumpAndSettle();

      expect(
        mockChat.reconnectCallCount,
        0,
        reason:
            'attemptReconnect should not be called on wifi→wifi (no offline transition)',
      );
    });

    testWidgets('does not call attemptReconnect when initial state is online', (
      tester,
    ) async {
      final connectivityController =
          StreamController<List<ConnectivityResult>>();
      addTearDown(connectivityController.close);

      final mockChat = _MockChatNotifier();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            connectivityProvider.overrideWith(
              (ref) => connectivityController.stream,
            ),
            chatProvider.overrideWith(() => mockChat),
          ],
          child: const MaterialApp(
            home: AgentEventListener(child: SizedBox.shrink()),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // First emission is online — _wasOffline starts as false.
      connectivityController.add([ConnectivityResult.wifi]);
      await tester.pumpAndSettle();

      expect(
        mockChat.reconnectCallCount,
        0,
        reason: 'attemptReconnect should not be called on initial online state',
      );
    });
  });
}
