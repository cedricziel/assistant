import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_app/api/models/server_profile.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';
import 'package:assistant_app/features/chat/chat_timeline_entry.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/notifications/notification_service.dart';
import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:assistant_app/features/settings/response_timing_preference.dart';
import 'package:assistant_app/features/settings/settings_screen.dart';
import 'package:assistant_app/features/settings/tool_permission_rules_provider.dart';
import 'package:assistant_app/shared/platform/adaptive_app.dart';
import 'package:assistant_app/shared/theme/assistant_theme.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:built_collection/built_collection.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shared_preferences/shared_preferences.dart';

// ---------------------------------------------------------------------------
// Fakes

class _FakeChatNotifier extends ChatNotifier {
  @override
  Future<ChatState> build() async => const ChatState();

  void push(ChatState s) => state = AsyncData(s);

  @override
  Future<void> loadConversation(String conversationId) async {}

  @override
  void dismissError() {}
}

class _AdaPersonasNotifier extends PersonasNotifier {
  @override
  Future<PersonasState> build() async => PersonasState(
    activePersona: PersonaSummary(
      (b) => b
        ..id = 'ada'
        ..name = 'Ada'
        ..description = '',
    ),
  );
}

class _FakeConversationListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => const ConversationListState();

  @override
  Future<void> refresh() async {}

  @override
  void prependConversation(ConversationSummary conv) {}
}

class _FakeShowResponseTimingNotifier extends ShowResponseTimingNotifier {
  _FakeShowResponseTimingNotifier(this.initial);

  bool initial;

  @override
  Future<bool> build() async => initial;
}

class _NoRulesNotifier extends ToolPermissionRulesNotifier {
  @override
  Future<List<ToolPermissionRuleSummary>> build() async => const [];
}

class _FakeNotificationService extends NotificationService {
  _FakeNotificationService({required super.ref});

  @override
  Future<void> initialize() async {}

  @override
  Future<bool> requestPermission() async => true;

  @override
  Future<void> show(
    String title,
    String body, {
    String? conversationId,
  }) async {}
}

// ---------------------------------------------------------------------------
// Fixture conversation

/// A conversation as it arrives from `GET /api/conversations/{id}`: a user
/// message with an attachment, an assistant reply with a code fence, a link
/// and timing, a tool call, and a stopped reply.
List<ChatMessage> _fixtureMessages() {
  final history = chatMessagesFromHistory([
    MessageSummary(
      (b) => b
        ..id = 'u1'
        ..role = 'user'
        ..content = 'Please review the report'
        ..turn = 1
        ..createdAt = DateTime(2026, 1, 1)
        ..ttsAvailable = false
        ..attachments = ListBuilder<AttachmentMetaResponse>([
          AttachmentMetaResponse(
            (a) => a
              ..id = 'att-1'
              ..filename = 'quarterly-report.pdf'
              ..mimeType = 'application/pdf'
              ..url = '/api/attachments/att-1'
              ..sizeBytes = 2048
              ..createdAt = DateTime(2026, 1, 1),
          ),
        ]),
    ),
    MessageSummary(
      (b) => b
        ..id = 'a1'
        ..role = 'assistant'
        ..content =
            'Here is the summary.\n\n```dart\nvoid main() {}\n```\n\n'
            'Read [the full report](https://example.com/report).'
        ..turn = 1
        ..createdAt = DateTime(2026, 1, 1)
        ..ttsAvailable = false
        ..timing.replace(
          MessageTimingSummary(
            (t) => t
              ..totalMs = 12400
              ..timeToFirstTokenMs = 800
              ..outputTokens = 42
              ..decodeMs = 5200,
          ),
        ),
    ),
    MessageSummary(
      (b) => b
        ..id = 'a-tool'
        ..role = 'assistant'
        ..content = ''
        ..turn = 1
        ..createdAt = DateTime(2026, 1, 1)
        ..ttsAvailable = false
        ..toolCalls = ListBuilder<ToolCallSummary>([
          ToolCallSummary(
            (t) => t
              ..name = 'web-search'
              ..status = 'ok'
              ..result = '3 results',
          ),
        ]),
    ),
    MessageSummary(
      (b) => b
        ..id = 'a2'
        ..role = 'assistant'
        ..content = 'This reply was stopped.'
        ..turn = 1
        ..createdAt = DateTime(2026, 1, 1)
        ..ttsAvailable = false
        ..finishReason = 'cancelled',
    ),
  ]);

  // The stopped reply is the last one; put the thinking entry before the
  // first assistant bubble.
  return [
    history[0],
    ChatMessage(
      id: 'thinking-1',
      role: 'assistant',
      content: '',
      timelineType: TimelineEntryType.thinking,
      thinkingContent: 'I should open the report first.',
    ),
    history[1],
    history[2],
    history[3],
  ];
}

ChatState _fixtureState() => ChatState(
  conversationId: 'c1',
  messages: _fixtureMessages(),
  error: 'The connection dropped.',
);

// ---------------------------------------------------------------------------
// Pumping

Future<_FakeChatNotifier> _pumpChatScreen(
  WidgetTester tester, {
  required Brightness brightness,
  bool showResponseTiming = true,
}) async {
  final chat = _FakeChatNotifier();
  final pref = showResponseTiming
      ? _FakeShowResponseTimingNotifier(true)
      : null;
  tester.view.physicalSize = const Size(480, 1600);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        chatProvider.overrideWith(() => chat),
        personasProvider.overrideWith(_AdaPersonasNotifier.new),
        conversationListProvider.overrideWith(
          _FakeConversationListNotifier.new,
        ),
        if (pref != null) showResponseTimingProvider.overrideWith(() => pref),
        activeProfileProvider.overrideWithValue(
          const ServerProfile(baseUrl: 'http://localhost', token: 'test-token'),
        ),
      ],
      child: MaterialApp(
        theme: assistantLightTheme(),
        darkTheme: assistantDarkTheme(),
        themeMode: brightness == Brightness.dark
            ? ThemeMode.dark
            : ThemeMode.light,
        home: const ChatScreen(conversationId: 'c1'),
        builder: (_, child) => LegacyDesignBridge(child: child!),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return chat;
}

Future<void> _pumpSettings(
  WidgetTester tester, {
  required Brightness brightness,
}) async {
  tester.view.physicalSize = const Size(480, 1200);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        notificationServiceProvider.overrideWith(
          (ref) => _FakeNotificationService(ref: ref),
        ),
        toolPermissionRulesProvider.overrideWith(_NoRulesNotifier.new),
      ],
      child: MaterialApp(
        theme: assistantLightTheme(),
        darkTheme: assistantDarkTheme(),
        themeMode: brightness == Brightness.dark
            ? ThemeMode.dark
            : ThemeMode.light,
        home: const SettingsScreen(),
        builder: (_, child) => LegacyDesignBridge(child: child!),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

// ---------------------------------------------------------------------------
// Guideline helpers

/// Every tappable semantics node must carry a non-empty label.
void _expectTappableNodesAreLabelled(WidgetTester tester) {
  final root = tester.getSemantics(find.byType(ChatScreen));
  final unlabelled = <String>[];
  void walk(SemanticsNode node) {
    final data = node.getSemanticsData();
    // Mirror the guideline: hidden or merged nodes are not presented to the
    // user as separate targets.
    if (node.isInvisible ||
        node.isMergedIntoParent ||
        data.flagsCollection.isHidden ||
        data.flagsCollection.isTextField) {
      return;
    }
    final tappable =
        data.hasAction(SemanticsAction.tap) ||
        data.hasAction(SemanticsAction.longPress);
    // A tooltip is an accessible name too: an icon-only button announces it.
    if (tappable && data.label.trim().isEmpty && data.tooltip.trim().isEmpty) {
      unlabelled.add(
        'rect=${node.rect} actions=${data.actions} hint="${data.hint}"',
      );
    }
    node.visitChildren((child) {
      walk(child);
      return true;
    });
  }

  walk(root);
  expect(unlabelled, isEmpty, reason: 'unlabelled tap targets found');
}

void main() {
  setUp(() {
    SharedPreferences.setMockInitialValues({});
  });

  group('Chat screen accessibility guidelines', () {
    for (final brightness in Brightness.values) {
      final name = brightness == Brightness.dark ? 'dark' : 'light';

      testWidgets('pass in the $name theme with timing shown', (tester) async {
        final handle = tester.ensureSemantics();
        final chat = await _pumpChatScreen(tester, brightness: brightness);
        chat.push(_fixtureState());
        await tester.pumpAndSettle();

        // The fixture really exercises the surfaces the suite is about.
        expect(find.byType(CodeBlock), findsOneWidget);
        expect(find.byType(MessageTimingIndicator), findsOneWidget);
        expect(find.text('quarterly-report.pdf'), findsOneWidget);
        expect(
          find.byType(ChatTimelineEntry),
          findsNWidgets(2),
          reason: 'the tool call and the thinking entry render',
        );
        expect(find.byType(ThreadErrorBanner), findsOneWidget);
        expect(
          tester
              .getSemantics(find.byType(ThreadErrorBanner))
              .flagsCollection
              .isLiveRegion,
          isTrue,
          reason: 'an error banner is announced when it appears',
        );

        expect(chat.state.value!.messages.last.status, MessageStatus.stopped);
        expect(
          find.bySemanticsLabel(RegExp('^Ada, stopped\$')),
          findsOneWidget,
          reason: 'the persisted stopped finish_reason reaches the label',
        );

        await expectLater(tester, meetsGuideline(labeledTapTargetGuideline));
        await expectLater(tester, meetsGuideline(textContrastGuideline));
        _expectTappableNodesAreLabelled(tester);
        handle.dispose();
      });
    }

    testWidgets('pass with the timing preference at its default', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      // No provider override: the real preference reads as off.
      final chat = await _pumpChatScreen(
        tester,
        brightness: Brightness.light,
        showResponseTiming: false,
      );
      chat.push(_fixtureState());
      await tester.pumpAndSettle();

      expect(find.byType(MessageTimingIndicator), findsNothing);

      await expectLater(tester, meetsGuideline(labeledTapTargetGuideline));
      await expectLater(tester, meetsGuideline(textContrastGuideline));
      _expectTappableNodesAreLabelled(tester);
      handle.dispose();
    });
  });

  group('Settings accessibility guidelines', () {
    for (final brightness in Brightness.values) {
      final name = brightness == Brightness.dark ? 'dark' : 'light';

      testWidgets('the timing switch is labelled and meets them in $name', (
        tester,
      ) async {
        final handle = tester.ensureSemantics();
        await _pumpSettings(tester, brightness: brightness);

        final row = find.ancestor(
          of: find.text('Show response timing'),
          matching: find.byType(SwitchListTile),
        );
        expect(row, findsOneWidget);
        expect(
          tester.getSemantics(row).label,
          startsWith('Show response timing'),
        );

        await expectLater(tester, meetsGuideline(labeledTapTargetGuideline));
        handle.dispose();
      });
    }
  });
}
