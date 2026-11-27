import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/notifications/agent_event_listener.dart';
import 'package:assistant_app/features/notifications/notification_preferences.dart';
import 'package:assistant_app/features/notifications/notification_service.dart';
import 'package:flutter/foundation.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

// ---------------------------------------------------------------------------
// Fake notification service that records show() calls.

class _FakeNotificationService extends NotificationService {
  _FakeNotificationService({required super.ref});

  final List<({String title, String body, String? conversationId})> calls = [];

  /// Pretend the browser already holds a Web Push subscription.
  bool activePushSubscription = false;

  @override
  Future<void> initialize() async {}

  @override
  Future<bool> requestPermission() async => true;

  @override
  Future<bool> hasActivePushSubscription() async => activePushSubscription;

  @override
  Future<void> show(String title, String body, {String? conversationId}) async {
    calls.add((title: title, body: body, conversationId: conversationId));
  }
}

// ---------------------------------------------------------------------------
// Fake chat notifier that lets tests push state changes.

class _FakeChatNotifier extends ChatNotifier {
  @override
  Future<ChatState> build() async => const ChatState();

  void push(ChatState s) => state = AsyncData(s);

  @override
  Future<void> attemptReconnect() async {}
}

// ---------------------------------------------------------------------------
// Prefs notifier that resolves synchronously (avoids async loading race).

class _SyncPrefsNotifier extends NotificationPreferencesNotifier {
  _SyncPrefsNotifier(this._prefs);

  final SharedPreferences _prefs;

  @override
  Future<NotificationPreferences> build() =>
      SynchronousFuture(NotificationPreferences(_prefs));
}

// ---------------------------------------------------------------------------

InteractionRecord _approvalRequest({
  String id = 'int-1',
  String? toolName = 'bash',
  InteractionKind kind = InteractionKind.toolApproval,
  List<QuestionItem> questions = const [],
}) => InteractionRecord(
  id: id,
  kind: kind,
  status: InteractionStatus.pending,
  createdAt: DateTime.utc(2026),
  expiresAt: DateTime.utc(2026, 1, 1, 0, 10),
  toolName: toolName,
  questions: questions,
);

void main() {
  late SharedPreferences mockPrefs;

  setUp(() async {
    SharedPreferences.setMockInitialValues({});
    mockPrefs = await SharedPreferences.getInstance();
  });

  /// Pumps [AgentEventListener] with fakes, waits for providers to settle,
  /// and returns the chat notifier + fake notification service.
  Future<({_FakeChatNotifier chatNotifier, _FakeNotificationService ns})> pump(
    WidgetTester tester,
  ) async {
    final chatNotifier = _FakeChatNotifier();

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          chatProvider.overrideWith(() => chatNotifier),
          // Use SynchronousFuture so prefs are immediately in AsyncData.
          notificationPreferencesProvider.overrideWith(
            () => _SyncPrefsNotifier(mockPrefs),
          ),
          notificationServiceProvider.overrideWith(
            (ref) => _FakeNotificationService(ref: ref),
          ),
        ],
        child: const MaterialApp(
          home: AgentEventListener(child: Scaffold(body: Text('content'))),
        ),
      ),
    );

    // One pump to let the SynchronousFuture-backed notifier settle.
    await tester.pump();

    // Directly read the service from the ProviderScope container so we have
    // a typed reference to the fake (initialises the lazy Provider).
    final container = ProviderScope.containerOf(
      tester.element(find.byType(AgentEventListener)),
    );
    final ns =
        container.read(notificationServiceProvider) as _FakeNotificationService;

    return (chatNotifier: chatNotifier, ns: ns);
  }

  testWidgets('fires "Skill complete" when tool result is success', (
    tester,
  ) async {
    final (:chatNotifier, :ns) = await pump(tester);

    chatNotifier.push(
      const ChatState(
        isSending: true,
        lastToolResult: ChatToolResult(toolName: 'my-skill', status: 'ok'),
      ),
    );
    await tester.pump();

    expect(ns.calls, hasLength(1));
    expect(ns.calls.first.title, equals('Skill complete'));
    expect(ns.calls.first.body, equals('my-skill'));
  });

  testWidgets('fires "Skill failed" when tool result is error', (tester) async {
    final (:chatNotifier, :ns) = await pump(tester);

    chatNotifier.push(
      const ChatState(
        isSending: true,
        lastToolResult: ChatToolResult(toolName: 'bad-skill', status: 'error'),
      ),
    );
    await tester.pump();

    expect(ns.calls, hasLength(1));
    expect(ns.calls.first.title, equals('Skill failed'));
    expect(ns.calls.first.body, equals('bad-skill'));
  });

  testWidgets('fires "Assistant error" on chat error', (tester) async {
    final (:chatNotifier, :ns) = await pump(tester);

    chatNotifier.push(const ChatState(error: 'Something went wrong'));
    await tester.pump();

    expect(ns.calls, hasLength(1));
    expect(ns.calls.first.title, equals('Assistant error'));
    expect(ns.calls.first.body, equals('Something went wrong'));
  });

  testWidgets(
    'shows "Approval needed" with the conversation when backgrounded',
    (tester) async {
      final (:chatNotifier, :ns) = await pump(tester);

      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
      await tester.pump();

      chatNotifier.push(
        ChatState(
          conversationId: 'conv-7',
          lastInteractionRequest: _approvalRequest(),
        ),
      );
      await tester.pump();
      await tester.pump();

      expect(ns.calls, hasLength(1));
      expect(ns.calls.first.title, equals('Approval needed'));
      expect(ns.calls.first.body, equals('bash'));
      expect(ns.calls.first.conversationId, equals('conv-7'));

      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    },
  );

  testWidgets('shows the question copy for an ask-user request', (
    tester,
  ) async {
    final (:chatNotifier, :ns) = await pump(tester);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pump();

    chatNotifier.push(
      ChatState(
        conversationId: 'conv-8',
        lastInteractionRequest: _approvalRequest(
          id: 'int-2',
          toolName: null,
          kind: InteractionKind.question,
          questions: const [
            QuestionItem(id: 'q1', prompt: 'Which environment?'),
          ],
        ),
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(ns.calls, hasLength(1));
    expect(ns.calls.first.title, equals('Your assistant has a question'));
    expect(ns.calls.first.body, equals('Which environment?'));
    expect(ns.calls.first.conversationId, equals('conv-8'));

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
  });

  testWidgets('stays silent when the app is resumed', (tester) async {
    final (:chatNotifier, :ns) = await pump(tester);

    chatNotifier.push(
      ChatState(
        conversationId: 'conv-7',
        lastInteractionRequest: _approvalRequest(),
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(ns.calls, isEmpty);
  });

  testWidgets('stays silent when approval notifications are off', (
    tester,
  ) async {
    await mockPrefs.setBool('notifications.approval_requests', false);
    final (:chatNotifier, :ns) = await pump(tester);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pump();

    chatNotifier.push(
      ChatState(
        conversationId: 'conv-7',
        lastInteractionRequest: _approvalRequest(),
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(ns.calls, isEmpty);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
  });

  testWidgets('stays silent on web with an active push subscription', (
    tester,
  ) async {
    final (:chatNotifier, :ns) = await pump(tester);
    ns.activePushSubscription = true;

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pump();

    chatNotifier.push(
      ChatState(
        conversationId: 'conv-7',
        lastInteractionRequest: _approvalRequest(),
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(ns.calls, isEmpty);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
  });
}
