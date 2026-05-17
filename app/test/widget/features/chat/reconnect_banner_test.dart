// Widget tests for [ReconnectBanner]. Pins:
//   - renders nothing when isReconnecting is false (the routine-resume
//     case must not flash a banner)
//   - renders the "Reconnecting…" label + spinner when true
//   - disappears as soon as the flag flips back to false
//
// Specified by: openspec/changes/chat-stream-progress-ux/specs/.../
//   "Reconnect banner during attemptReconnect"

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/reconnect_banner.dart';

class _FakeChatNotifier extends ChatNotifier {
  @override
  Future<ChatState> build() async => const ChatState();

  void push(ChatState s) => state = AsyncData(s);
}

Future<_FakeChatNotifier> _pumpBanner(WidgetTester tester) async {
  final notifier = _FakeChatNotifier();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [chatProvider.overrideWith(() => notifier)],
      child: const MaterialApp(home: Scaffold(body: ReconnectBanner())),
    ),
  );
  await tester.pump();
  return notifier;
}

void main() {
  group('ReconnectBanner', () {
    testWidgets('renders nothing when isReconnecting is false', (tester) async {
      await _pumpBanner(tester);

      expect(find.byKey(const Key('reconnect_banner')), findsNothing);
      expect(find.text('Reconnecting…'), findsNothing);
    });

    testWidgets('renders the banner when isReconnecting is true', (
      tester,
    ) async {
      final notifier = await _pumpBanner(tester);

      notifier.push(const ChatState(isReconnecting: true));
      await tester.pump();

      expect(find.byKey(const Key('reconnect_banner')), findsOneWidget);
      expect(find.text('Reconnecting…'), findsOneWidget);
    });

    testWidgets('disappears when isReconnecting flips back to false', (
      tester,
    ) async {
      final notifier = await _pumpBanner(tester);

      notifier.push(const ChatState(isReconnecting: true));
      await tester.pump();
      expect(find.byKey(const Key('reconnect_banner')), findsOneWidget);

      notifier.push(const ChatState(isReconnecting: false));
      await tester.pump();
      expect(find.byKey(const Key('reconnect_banner')), findsNothing);
    });

    testWidgets(
      'no spurious flash on routine resume (no isReconnecting flip)',
      (tester) async {
        final notifier = await _pumpBanner(tester);

        // Routine resume — state mutates without ever setting
        // isReconnecting. Banner stays hidden.
        notifier.push(const ChatState(isSending: true));
        await tester.pump();
        notifier.push(const ChatState(messages: [], pendingQueue: []));
        await tester.pump();

        expect(find.byKey(const Key('reconnect_banner')), findsNothing);
      },
    );
  });
}
