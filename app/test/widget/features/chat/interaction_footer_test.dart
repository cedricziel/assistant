// A tool approval renders in the footer of its tool call, next to an MCP App,
// and answers through the callback the chat screen supplies.

import 'package:assistant_app/api/models/interaction_answer.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/mcp_app/mcp_app_provider.dart';
import 'package:assistant_app/features/chat/chat_timeline_entry.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../../support/fake_mcp_app.dart';

InteractionRecord _approval({
  InteractionStatus status = InteractionStatus.pending,
  InteractionKind kind = InteractionKind.toolApproval,
  bool rendersCard = true,
  ApprovalDecision? decision,
}) => InteractionRecord(
  id: 'int-1',
  kind: kind,
  status: status,
  createdAt: DateTime.now(),
  expiresAt: DateTime.now().add(const Duration(minutes: 10)),
  toolCallId: 'call-1',
  toolName: 'bash',
  arguments: const {'command': 'git status'},
  options: const [ApprovalDecision.allowOnce, ApprovalDecision.rejectOnce],
  decision: decision,
  rendersCard: rendersCard,
);

ChatMessage _toolCall({InteractionRecord? interaction, McpApp? app}) =>
    ChatMessage(
      id: 'toolcall-bash',
      role: 'assistant',
      content: '',
      timelineType: TimelineEntryType.toolCall,
      toolCalls: [
        ToolCallRecord(
          toolName: 'bash',
          status: ToolCallStatus.pending,
          toolCallId: 'call-1',
          app: app,
        )..interaction = interaction,
      ],
    );

Future<void> _pump(
  WidgetTester tester,
  ChatMessage message, {
  void Function(String, InteractionAnswer)? onAnswer,
}) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        mcpAppSurfaceFactoryProvider.overrideWithValue(
          FakeMcpAppSurface.factory,
        ),
        mcpAppBackendProvider.overrideWith((ref, _) => FakeMcpAppBackend()),
      ],
      child: MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: ThreadDensityScope(
              density: TimelineDensity.normal,
              child: ChatTimelineEntry(
                message: message,
                conversationId: 'conv-1',
                entryState: timelineEntryState(message),
                onInteractionAnswer: onAnswer,
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pump();
}

void main() {
  group('timelineEntryState', () {
    test('a tool call with an open interaction is awaitingInput', () {
      expect(
        timelineEntryState(_toolCall(interaction: _approval())),
        EntryState.awaitingInput,
      );
      expect(
        timelineEntryState(
          _toolCall(
            interaction: _approval(status: InteractionStatus.submitting),
          ),
        ),
        EntryState.awaitingInput,
      );
    });

    test('an answered interaction no longer holds the entry open', () {
      final answered = _toolCall(
        interaction: _approval(status: InteractionStatus.resolved),
      );
      expect(timelineEntryState(answered), EntryState.complete);
    });

    test('keeps the active, stale and complete states otherwise', () {
      expect(timelineEntryState(_toolCall()), EntryState.complete);
      expect(
        timelineEntryState(_toolCall()..isStreaming = true),
        EntryState.active,
      );
      expect(timelineEntryState(_toolCall()..isStale = true), EntryState.stale);
    });
  });

  group('approval in the tool call footer', () {
    testWidgets('renders the card and Allow once reaches the callback', (
      tester,
    ) async {
      String? answeredId;
      InteractionAnswer? answered;
      await _pump(
        tester,
        _toolCall(interaction: _approval()),
        onAnswer: (id, answer) {
          answeredId = id;
          answered = answer;
        },
      );

      expect(find.byType(ApprovalCard), findsOneWidget);
      await tester.tap(find.text('Allow once'));
      await tester.pump();

      expect(answeredId, 'int-1');
      expect(
        (answered as ToolApprovalAnswer).decision,
        ApprovalDecision.allowOnce,
      );
    });

    testWidgets('shows a receipt with no buttons once answered', (
      tester,
    ) async {
      await _pump(
        tester,
        _toolCall(
          interaction: _approval(
            status: InteractionStatus.resolved,
            decision: ApprovalDecision.allowOnce,
          ),
        ),
      );

      expect(find.text('Allowed once'), findsOneWidget);
      expect(find.text('Allow once'), findsNothing);
    });

    testWidgets('keeps the MCP App frame mounted beside the card', (
      tester,
    ) async {
      await _pump(
        tester,
        _toolCall(
          interaction: _approval(),
          app: const McpApp(
            server: 'weather',
            resourceUri: 'ui://weather/forecast',
            html: '<p>Sunny</p>',
          ),
        ),
      );

      expect(find.byType(ApprovalCard), findsOneWidget);
      expect(find.byKey(fakeMcpAppViewKey), findsOneWidget);

      // Unmount so the frame's timers stop, as the MCP App tests do.
      await tester.pumpWidget(const SizedBox());
      await tester.pump(const Duration(seconds: 1));
    });

    testWidgets('a tool call without an interaction has no card', (
      tester,
    ) async {
      await _pump(tester, _toolCall());

      expect(find.byType(ApprovalCard), findsNothing);
    });

    testWidgets('an unregistered card kind shows the inert notice', (
      tester,
    ) async {
      await _pump(
        tester,
        _toolCall(interaction: _approval(kind: InteractionKind.unknown)),
      );

      expect(find.byType(UnknownInteractionCard), findsOneWidget);
      expect(find.byType(ApprovalCard), findsNothing);
    });

    testWidgets('a kind that renders no card shows nothing', (tester) async {
      await _pump(
        tester,
        _toolCall(
          interaction: _approval(
            kind: InteractionKind.unknown,
            rendersCard: false,
          ),
        ),
      );

      expect(find.byType(UnknownInteractionCard), findsNothing);
      expect(find.byType(ApprovalCard), findsNothing);
    });
  });

  group('interaction entry without a tool call', () {
    testWidgets('renders the card on its own', (tester) async {
      final entry = ChatMessage(
        id: 'interaction-int-1',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.interaction,
      )..interaction = _approval();

      await _pump(tester, entry);

      expect(find.byType(ApprovalCard), findsOneWidget);
    });
  });
}
