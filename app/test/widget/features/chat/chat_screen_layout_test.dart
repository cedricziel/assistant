import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';
import 'package:assistant_app/features/chat/conversation_list.dart';
import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:assistant_app/features/pwa/pwa_provider.dart';
import 'package:assistant_app/shared/nav_shell.dart';
import 'package:assistant_app/shared/platform/adaptive_app.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';

class _FakeChatNotifier extends ChatNotifier {
  @override
  Future<ChatState> build() async => const ChatState();
}

class _FakePersonasNotifier extends PersonasNotifier {
  @override
  Future<PersonasState> build() async => const PersonasState();
}

class _FakeConversationListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => const ConversationListState();

  @override
  Future<void> refresh() async {}
}

class _FakePwaNotifier extends PwaInstallNotifier {
  @override
  bool build() => false;
}

/// Pumps [ChatScreen] inside the real [NavShell], at [width] logical pixels.
Future<void> _pump(
  WidgetTester tester, {
  required double width,
  bool collapsed = false,
}) async {
  SharedPreferences.setMockInitialValues({
    'assistant.sidebarCollapsed': collapsed,
  });
  tester.view.physicalSize = Size(width, 900);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);

  final router = GoRouter(
    initialLocation: '/chat',
    routes: [
      ShellRoute(
        builder: (_, _, child) => NavShell(child: child),
        routes: [GoRoute(path: '/chat', builder: (_, _) => const ChatScreen())],
      ),
    ],
  );
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        chatProvider.overrideWith(_FakeChatNotifier.new),
        personasProvider.overrideWith(_FakePersonasNotifier.new),
        conversationListProvider.overrideWith(
          _FakeConversationListNotifier.new,
        ),
        pwaInstallProvider.overrideWith(_FakePwaNotifier.new),
      ],
      child: MaterialApp.router(
        routerConfig: router,
        builder: (_, child) => LegacyDesignBridge(child: child!),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

/// The chat's own [Scaffold], which is the one carrying the app bar.
Scaffold _chatScaffold(WidgetTester tester) => tester.widget<Scaffold>(
  find
      .descendant(of: find.byType(ChatScreen), matching: find.byType(Scaffold))
      .first,
);

/// Whether the conversation list is inline in the chat body.
bool _listInline(WidgetTester tester) => find
    .descendant(
      of: find.byType(ChatScreen),
      matching: find.byType(ConversationListPane),
    )
    .evaluate()
    .isNotEmpty;

bool _hasDrawer(WidgetTester tester) => _chatScaffold(tester).drawer != null;

Finder get _menuButton =>
    find.descendant(of: find.byType(AppBar), matching: find.byIcon(Icons.menu));

void main() {
  group('ChatScreen conversation list vs pane width', () {
    testWidgets('740 px window: list in the drawer, not inline', (
      tester,
    ) async {
      await _pump(tester, width: 740);

      expect(_listInline(tester), isFalse);
      expect(_hasDrawer(tester), isTrue);
    });

    testWidgets('900 px with the expanded sidebar: drawer and menu button', (
      tester,
    ) async {
      await _pump(tester, width: 900);

      expect(_listInline(tester), isFalse);
      expect(_hasDrawer(tester), isTrue);
      expect(_menuButton, findsOneWidget);
    });

    testWidgets('840 px with the collapsed sidebar: drawer (pane 767)', (
      tester,
    ) async {
      await _pump(tester, width: 840, collapsed: true);

      expect(_listInline(tester), isFalse);
      expect(_hasDrawer(tester), isTrue);
    });

    testWidgets('841 px with the collapsed sidebar: inline (pane 768)', (
      tester,
    ) async {
      await _pump(tester, width: 841, collapsed: true);

      expect(_listInline(tester), isTrue);
      expect(_hasDrawer(tester), isFalse);
      expect(_menuButton, findsNothing);
    });

    testWidgets('1280 px with the expanded sidebar: inline, no drawer', (
      tester,
    ) async {
      await _pump(tester, width: 1280);

      expect(_listInline(tester), isTrue);
      expect(_hasDrawer(tester), isFalse);
    });

    testWidgets('expanding the sidebar at 900 px flips inline to drawer once', (
      tester,
    ) async {
      await _pump(tester, width: 900, collapsed: true);
      expect(_listInline(tester), isTrue, reason: 'pane is 827 px collapsed');

      final flips = <bool>[];
      var last = _listInline(tester);
      await tester.tap(find.byTooltip('Expand sidebar'));
      for (var elapsed = 0; elapsed <= 400; elapsed += 16) {
        await tester.pump(const Duration(milliseconds: 16));
        final now = _listInline(tester);
        if (now != last) flips.add(now);
        last = now;
      }
      await tester.pumpAndSettle();

      expect(tester.takeException(), isNull);
      expect(flips, [false], reason: 'exactly one inline → drawer flip');
      expect(_listInline(tester), isFalse);
      expect(_hasDrawer(tester), isTrue);
    });
  });

  group('ChatScreen thread density', () {
    testWidgets('is computed from the thread width, not the window', (
      tester,
    ) async {
      // 1000 px window, expanded sidebar: pane 759 px, list in the drawer,
      // so the thread is 759 px wide and resolves expanded (> 700).
      // 1000 px collapsed: pane 927, inline 240 + 1 → thread 686 → normal.
      await _pump(tester, width: 1000, collapsed: true);
      final scope = find.descendant(
        of: find.byType(ChatScreen),
        matching: find.byType(ThreadDensityScope),
      );
      expect(
        tester.widget<ThreadDensityScope>(scope.first).density,
        TimelineDensity.normal,
        reason: 'window 1000 would say expanded; thread is only 686 px',
      );
    });
  });
}
