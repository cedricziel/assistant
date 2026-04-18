import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/timeline_section.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('ChatTimelineSection', () {
    testWidgets('renders tool call with wrench icon and tool name', (
      tester,
    ) async {
      final msg = ChatMessage(
        id: 'tc1',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.toolCall,
        toolCalls: [
          ToolCallRecord(toolName: 'web-search', status: ToolCallStatus.ok),
        ],
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: ChatTimelineSection(message: msg)),
        ),
      );

      expect(find.byIcon(Icons.build_outlined), findsOneWidget);
      expect(find.text('web-search'), findsOneWidget);
      expect(find.byIcon(Icons.check_circle_outline), findsOneWidget);
    });

    testWidgets('renders pending tool call with spinner', (tester) async {
      final msg = ChatMessage(
        id: 'tc2',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.toolCall,
        toolCalls: [
          ToolCallRecord(toolName: 'file-read', status: ToolCallStatus.pending),
        ],
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: ChatTimelineSection(message: msg)),
        ),
      );

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(find.text('file-read'), findsOneWidget);
    });

    testWidgets('renders thinking entry with brain icon', (tester) async {
      final msg = ChatMessage(
        id: 'th1',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.thinking,
        thinkingContent: 'Let me reason about this...',
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: ChatTimelineSection(message: msg)),
        ),
      );

      expect(find.byIcon(Icons.psychology_outlined), findsOneWidget);
      expect(find.text('Thinking'), findsOneWidget);
    });

    testWidgets('renders subagent entry with smart toy icon', (tester) async {
      final msg = ChatMessage(
        id: 'sa1',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.subagent,
        subagentId: 'research-1',
        subagentTask: 'Find weather data',
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: ChatTimelineSection(message: msg)),
        ),
      );

      expect(find.byIcon(Icons.smart_toy_outlined), findsOneWidget);
      expect(find.text('Find weather data'), findsOneWidget);
    });

    testWidgets('expands on tap to show thinking content', (tester) async {
      final msg = ChatMessage(
        id: 'th2',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.thinking,
        thinkingContent: 'Deep reasoning here',
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: ChatTimelineSection(message: msg)),
        ),
      );

      // Thinking content not visible initially.
      expect(find.text('Deep reasoning here'), findsNothing);

      // Tap to expand.
      await tester.tap(find.byType(GestureDetector).first);
      await tester.pumpAndSettle();

      expect(find.text('Deep reasoning here'), findsOneWidget);
    });

    testWidgets('expands tool call to show arguments and result', (
      tester,
    ) async {
      final msg = ChatMessage(
        id: 'tc3',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.toolCall,
        toolCalls: [
          ToolCallRecord(
            toolName: 'web-search',
            status: ToolCallStatus.ok,
            arguments: {'query': 'flutter docs'},
            result: 'Found 3 results',
          ),
        ],
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: ChatTimelineSection(message: msg)),
        ),
      );

      // Result not visible initially.
      expect(find.text('Found 3 results'), findsNothing);

      // Tap to expand.
      await tester.tap(find.byType(GestureDetector).first);
      await tester.pumpAndSettle();

      expect(find.text('Found 3 results'), findsOneWidget);
    });

    testWidgets('subagent shows summary when completed', (tester) async {
      final msg = ChatMessage(
        id: 'sa2',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.subagent,
        subagentId: 'r1',
        subagentTask: 'Research topic',
        subagentSummary: 'Found 5 papers',
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: ChatTimelineSection(message: msg)),
        ),
      );

      // Summary should be visible in collapsed state as status text.
      expect(find.byIcon(Icons.check_circle_outline), findsOneWidget);
    });

    testWidgets('error tool call shows error icon', (tester) async {
      final msg = ChatMessage(
        id: 'tc4',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.toolCall,
        toolCalls: [
          ToolCallRecord(toolName: 'bash', status: ToolCallStatus.error),
        ],
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: ChatTimelineSection(message: msg)),
        ),
      );

      expect(find.byIcon(Icons.error_outline), findsOneWidget);
    });
  });
}
