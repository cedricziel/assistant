import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/conversation_list.dart';
import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

/// Records the `title` argument of every [createConversation] call.
class _FakeConversationListNotifier extends ConversationListNotifier {
  final List<String?> titles = [];

  @override
  Future<ConversationListState> build() async => const ConversationListState();

  @override
  Future<String?> createConversation({String? title}) async {
    titles.add(title);
    return 'new-convo-123';
  }
}

class _SeededConversationListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => ConversationListState(
    conversations: [
      ConversationListEntry(
        id: 'p1',
        title: 'Pinned chat',
        createdAt: DateTime.utc(2024, 1, 1),
        updatedAt: DateTime.utc(2024, 1, 1),
        pinnedAt: DateTime.utc(2024, 2, 1),
      ),
      ConversationListEntry(
        id: 'r1',
        title: 'Recent chat',
        createdAt: DateTime.utc(2024, 1, 1),
        updatedAt: DateTime.utc(2024, 3, 1),
      ),
    ],
  );
}

/// Seeded list that records row-action calls.
class _RecordingConversationListNotifier
    extends _SeededConversationListNotifier {
  _RecordingConversationListNotifier({this.error});

  final String? error;
  final List<(String, String)> renames = [];
  final List<(String, bool)> pins = [];
  final List<String> deletes = [];

  @override
  Future<String?> renameConversation(String id, String title) async {
    renames.add((id, title));
    return error;
  }

  @override
  Future<String?> setPinned(String id, bool pinned) async {
    pins.add((id, pinned));
    return error;
  }

  @override
  Future<void> deleteConversation(String id) async => deletes.add(id);
}

class _FakeChatNotifier extends ChatNotifier {
  @override
  Future<ChatState> build() async => const ChatState();
}

final popupItems = find.byWidgetPredicate((w) => w is PopupMenuItem);

void main() {
  testWidgets('New Chat button creates a conversation without a title', (
    tester,
  ) async {
    final notifier = _FakeConversationListNotifier();
    final router = GoRouter(
      routes: [
        GoRoute(
          path: '/',
          builder: (_, _) => const Scaffold(body: ConversationListPane()),
        ),
        GoRoute(
          path: '/chat/:id',
          builder: (_, _) => const Scaffold(body: Text('Chat')),
        ),
      ],
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          conversationListProvider.overrideWith(() => notifier),
          chatProvider.overrideWith(_FakeChatNotifier.new),
        ],
        child: MaterialApp.router(routerConfig: router),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('new_chat_button')));
    await tester.pumpAndSettle();

    expect(notifier.titles, [
      null,
    ], reason: 'An explicit title would lock the row against auto-titling');
  });

  testWidgets('shows a Pinned group and highlights the selected conversation', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          conversationListProvider.overrideWith(
            _SeededConversationListNotifier.new,
          ),
          chatProvider.overrideWith(_FakeChatNotifier.new),
        ],
        child: const MaterialApp(
          home: Scaffold(body: ConversationListPane(selectedId: 'r1')),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Pinned'), findsOneWidget);
    final tiles = tester.widgetList<ListTile>(find.byType(ListTile)).toList();
    expect(tiles.map((t) => t.selected), [false, true]);
  });

  group('row actions', () {
    late _RecordingConversationListNotifier notifier;

    Future<void> pumpPane(
      WidgetTester tester, {
      String? selectedId,
      String? error,
    }) async {
      notifier = _RecordingConversationListNotifier(error: error);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            conversationListProvider.overrideWith(() => notifier),
            chatProvider.overrideWith(_FakeChatNotifier.new),
          ],
          child: MaterialApp(
            home: Scaffold(body: ConversationListPane(selectedId: selectedId)),
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    Future<void> rightClick(WidgetTester tester, String title) async {
      await tester.tapAt(
        tester.getCenter(find.text(title)),
        buttons: kSecondaryMouseButton,
        kind: PointerDeviceKind.mouse,
      );
      await tester.pumpAndSettle();
    }

    Future<void> longPress(
      WidgetTester tester,
      String title, {
      PointerDeviceKind kind = PointerDeviceKind.touch,
    }) async {
      final gesture = await tester.startGesture(
        tester.getCenter(find.text(title)),
        kind: kind,
      );
      await tester.pump(kLongPressTimeout + const Duration(milliseconds: 50));
      await gesture.up();
      await tester.pumpAndSettle();
    }

    /// Tabs until the overflow button has focus, then presses Enter.
    Future<void> activateOverflowWithKeyboard(WidgetTester tester) async {
      final button = tester.element(find.byTooltip('Conversation actions'));
      var focused = false;
      for (var i = 0; i < 10 && !focused; i++) {
        await tester.sendKeyEvent(LogicalKeyboardKey.tab);
        await tester.pump();
        FocusManager.instance.primaryFocus?.context?.visitAncestorElements((e) {
          focused = focused || e == button;
          return !focused;
        });
      }
      expect(focused, isTrue);
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pumpAndSettle();
    }

    Future<void> withPlatform(
      TargetPlatform platform,
      Future<void> Function() body,
    ) async {
      debugDefaultTargetPlatformOverride = platform;
      try {
        await body();
      } finally {
        debugDefaultTargetPlatformOverride = null;
      }
    }

    testWidgets('a right-click opens a menu at the pointer with the actions', (
      tester,
    ) async {
      await pumpPane(tester);

      await rightClick(tester, 'Recent chat');

      expect(popupItems, findsNWidgets(3));
      expect(find.text('Rename'), findsOneWidget);
      expect(find.text('Pin'), findsOneWidget);
      expect(find.text('Delete'), findsOneWidget);
      expect(find.byType(BottomSheet), findsNothing);
      expect(find.byType(CupertinoActionSheet), findsNothing);
    });

    testWidgets('a pinned conversation offers Unpin', (tester) async {
      await pumpPane(tester);

      await rightClick(tester, 'Pinned chat');
      expect(find.text('Unpin'), findsOneWidget);
      expect(find.text('Pin'), findsNothing);

      await tester.tap(find.text('Unpin'));
      await tester.pumpAndSettle();
      expect(notifier.pins, [('p1', false)]);
    });

    testWidgets('Pin pins the row', (tester) async {
      await pumpPane(tester);

      await rightClick(tester, 'Recent chat');
      await tester.tap(find.text('Pin'));
      await tester.pumpAndSettle();

      expect(notifier.pins, [('r1', true)]);
    });

    testWidgets('an overflow click opens a menu anchored to the button', (
      tester,
    ) async {
      await pumpPane(tester, selectedId: 'r1');

      final button = find.byTooltip('Conversation actions');
      final rect = tester.getRect(find.byType(IconButton));
      await tester.tap(button, kind: PointerDeviceKind.mouse);
      await tester.pumpAndSettle();

      expect(find.text('Rename'), findsOneWidget);
      expect(find.byType(BottomSheet), findsNothing);
      final menuTop = tester.getTopLeft(popupItems.first);
      expect(menuTop.dy, greaterThanOrEqualTo(rect.bottom - 1));
    });

    testWidgets('a touch long-press opens an action sheet on Android', (
      tester,
    ) async {
      await pumpPane(tester);

      await longPress(tester, 'Recent chat');

      expect(find.byType(BottomSheet), findsOneWidget);
      expect(popupItems, findsNothing);
      expect(find.text('Rename'), findsOneWidget);
      expect(find.text('Pin'), findsOneWidget);
      expect(find.text('Delete'), findsOneWidget);
    });

    testWidgets('a touch long-press opens an action sheet on the web', (
      tester,
    ) async {
      // Mobile web: Material, not a Cupertino platform. A finger must get a
      // sheet, not a desktop menu.
      await withPlatform(TargetPlatform.android, () async {
        await pumpPane(tester);
        await longPress(tester, 'Recent chat');
        expect(find.byType(BottomSheet), findsOneWidget);
        expect(popupItems, findsNothing);
      });
    });

    testWidgets('a touch long-press opens a Cupertino action sheet on iOS', (
      tester,
    ) async {
      await withPlatform(TargetPlatform.iOS, () async {
        await pumpPane(tester);
        await longPress(tester, 'Recent chat');
        expect(find.byType(CupertinoActionSheet), findsOneWidget);
        expect(find.text('Rename'), findsOneWidget);
      });
    });

    testWidgets('keyboard activation opens an anchored menu on macOS', (
      tester,
    ) async {
      await withPlatform(TargetPlatform.macOS, () async {
        await pumpPane(tester, selectedId: 'r1');
        await activateOverflowWithKeyboard(tester);
        expect(popupItems, findsWidgets);
        expect(find.byType(BottomSheet), findsNothing);
      });
    });

    testWidgets('keyboard activation opens an action sheet on Android', (
      tester,
    ) async {
      await withPlatform(TargetPlatform.android, () async {
        await pumpPane(tester, selectedId: 'r1');
        await activateOverflowWithKeyboard(tester);
        expect(find.byType(BottomSheet), findsOneWidget);
        expect(popupItems, findsNothing);
      });
    });

    testWidgets('Rename opens the dialog and renames the conversation', (
      tester,
    ) async {
      await pumpPane(tester);

      await rightClick(tester, 'Recent chat');
      await tester.tap(find.text('Rename'));
      await tester.pumpAndSettle();

      expect(find.byType(AlertDialog), findsOneWidget);
      await tester.enterText(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.byType(TextField),
        ),
        'Quarterly plan',
      );
      await tester.pump();
      await tester.tap(find.widgetWithText(TextButton, 'Rename'));
      await tester.pumpAndSettle();

      expect(notifier.renames, [('r1', 'Quarterly plan')]);
    });

    testWidgets('cancelling Rename sends nothing', (tester) async {
      await pumpPane(tester);

      await rightClick(tester, 'Recent chat');
      await tester.tap(find.text('Rename'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(notifier.renames, isEmpty);
    });

    testWidgets('a failed PATCH shows a snack bar', (tester) async {
      await pumpPane(tester, error: 'title is too long');

      await rightClick(tester, 'Recent chat');
      await tester.tap(find.text('Pin'));
      await tester.pumpAndSettle();

      expect(find.byType(SnackBar), findsOneWidget);
      expect(find.textContaining('title is too long'), findsOneWidget);
    });

    testWidgets('Delete from the menu confirms, then deletes', (tester) async {
      await pumpPane(tester);

      await rightClick(tester, 'Recent chat');
      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();
      expect(find.text('Delete conversation?'), findsOneWidget);
      expect(notifier.deletes, isEmpty);

      await tester.tap(find.widgetWithText(TextButton, 'Delete'));
      await tester.pumpAndSettle();

      expect(notifier.deletes, ['r1']);
    });
  });
}
