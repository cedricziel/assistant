import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/pwa/pwa_provider.dart';
import 'package:assistant_app/shared/nav_shell.dart';

/// Tracks calls to [createConversation] and returns a predetermined ID.
class _FakeConversationListNotifier extends AsyncNotifier<ConversationListState>
    implements ConversationListNotifier {
  int createCount = 0;
  String? nextId = 'new-convo-123';

  /// How long to delay the response (simulates network latency for the
  /// debounce test).
  Duration delay = Duration.zero;

  @override
  Future<ConversationListState> build() async {
    return const ConversationListState();
  }

  @override
  Future<String?> createConversation({String? title}) async {
    createCount++;
    if (delay > Duration.zero) await Future<void>.delayed(delay);
    return nextId;
  }

  // Stubs for the rest of the interface — not exercised by these tests.
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _FakePwaNotifier extends PwaInstallNotifier {
  @override
  bool build() => false;
}

/// The route that the router navigated to last.
String? _lastLocation;

Widget _buildTestApp(_FakeConversationListNotifier fakeNotifier) {
  _lastLocation = null;
  final router = GoRouter(
    initialLocation: '/skills',
    routes: [
      ShellRoute(
        builder: (ctx, state, child) => NavShell(child: child),
        routes: [
          GoRoute(
            path: '/chat',
            builder: (_, state) {
              return const Scaffold(body: Text('Chat'));
            },
            routes: [
              GoRoute(
                path: ':id',
                builder: (_, state) {
                  _lastLocation = '/chat/${state.pathParameters['id']}';
                  return Scaffold(
                    body: Text('Chat ${state.pathParameters['id']}'),
                  );
                },
              ),
            ],
          ),
          GoRoute(
            path: '/skills',
            builder: (_, _) => const Scaffold(body: Text('Skills')),
          ),
          GoRoute(
            path: '/workflows',
            builder: (_, _) => const Scaffold(body: Text('Workflows')),
          ),
        ],
      ),
    ],
  );

  return ProviderScope(
    overrides: [
      conversationListProvider.overrideWith(() => fakeNotifier),
      pwaInstallProvider.overrideWith(_FakePwaNotifier.new),
    ],
    child: MaterialApp.router(routerConfig: router),
  );
}

/// Sends Ctrl+N — the new-chat shortcut on non-Apple platforms (the default
/// in Flutter widget tests, which run as TargetPlatform.android).
Future<void> _sendCtrlN(WidgetTester tester) async {
  await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
  await tester.sendKeyDownEvent(LogicalKeyboardKey.keyN);
  await tester.sendKeyUpEvent(LogicalKeyboardKey.keyN);
  await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
}

void main() {
  group('Cmd+N new chat shortcut', () {
    testWidgets('creates a new conversation and navigates to it', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1280, 900);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      final notifier = _FakeConversationListNotifier();
      await tester.pumpWidget(_buildTestApp(notifier));
      await tester.pumpAndSettle();

      // Verify we start on /skills (visible in both nav and page content).
      expect(find.text('Skills'), findsWidgets);

      // Simulate Ctrl+N (default test platform is Android, so control is used).
      await _sendCtrlN(tester);
      await tester.pumpAndSettle();

      expect(
        notifier.createCount,
        1,
        reason: 'Should create exactly one conversation',
      );
      expect(
        _lastLocation,
        '/chat/new-convo-123',
        reason: 'Should navigate to the new conversation',
      );
    });

    testWidgets(
      'rapid double-press creates only one conversation (debounce guard)',
      (tester) async {
        tester.view.physicalSize = const Size(1280, 900);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.reset);

        final notifier = _FakeConversationListNotifier()
          ..delay = const Duration(milliseconds: 200);
        await tester.pumpWidget(_buildTestApp(notifier));
        await tester.pumpAndSettle();

        // First Ctrl+N — starts an in-flight creation.
        await _sendCtrlN(tester);

        // Pump a frame so the callback fires, but don't settle (creation still in-flight).
        await tester.pump();

        // Second Ctrl+N — should be ignored because _creatingChat is true.
        await _sendCtrlN(tester);

        // Let the 200ms delay complete so no pending timers remain.
        await tester.pump(const Duration(milliseconds: 300));
        await tester.pumpAndSettle();

        expect(
          notifier.createCount,
          1,
          reason: 'Second Ctrl+N during in-flight should be ignored',
        );
      },
    );
  });
}
