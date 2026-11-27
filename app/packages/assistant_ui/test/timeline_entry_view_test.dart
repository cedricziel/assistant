import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

ChatMessage _message(
  TimelineEntryType type, {
  String? thinkingContent,
  List<ToolCallRecord> toolCalls = const [],
  String? summary,
  String? commandName,
  String? commandAckText,
}) => ChatMessage(
  id: 'entry',
  role: 'assistant',
  content: '',
  timelineType: type,
  thinkingContent: thinkingContent,
  toolCalls: toolCalls,
  subagentTask: 'Research goldens',
  subagentSummary: summary,
  commandName: commandName,
  commandAckText: commandAckText,
);

Widget _view(
  ChatMessage message, {
  EntryState entryState = EntryState.complete,
  Widget? footer,
  TimelineDensity density = TimelineDensity.normal,
}) => MaterialApp(
  home: ThreadDensityScope(
    density: density,
    child: Scaffold(
      body: SingleChildScrollView(
        child: TimelineEntryView(
          message: message,
          entryState: entryState,
          footer: footer,
        ),
      ),
    ),
  ),
);

void main() {
  group('TimelineEntryView dispatches on the entry type', () {
    testWidgets('thinking maps to ThinkingBody inside a shell', (tester) async {
      await tester.pumpWidget(
        _view(
          _message(
            TimelineEntryType.thinking,
            thinkingContent: 'Weighed two options.',
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(ThinkingBody), findsOneWidget);
      expect(find.byType(TimelineEntryShell), findsOneWidget);
      expect(find.text('Thought'), findsOneWidget);
    });

    testWidgets('tool call maps to ToolCallBody inside a shell', (
      tester,
    ) async {
      await tester.pumpWidget(
        _view(
          _message(
            TimelineEntryType.toolCall,
            toolCalls: [
              ToolCallRecord(
                toolName: 'web-search',
                status: ToolCallStatus.ok,
                result: '3 results',
              ),
            ],
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(ToolCallBody), findsOneWidget);
      expect(find.byType(TimelineEntryShell), findsOneWidget);
      expect(find.text('web-search'), findsOneWidget);
    });

    testWidgets('subagent maps to SubagentBody inside a shell', (tester) async {
      await tester.pumpWidget(
        _view(
          _message(
            TimelineEntryType.subagent,
            summary: 'Found three prior approaches.',
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(SubagentBody), findsOneWidget);
      expect(find.byType(TimelineEntryShell), findsOneWidget);
      expect(find.text('Research goldens'), findsOneWidget);
    });

    testWidgets('command maps to CommandBody without a shell', (tester) async {
      await tester.pumpWidget(
        _view(
          _message(
            TimelineEntryType.command,
            commandName: 'model',
            commandAckText: 'Switched to claude-sonnet',
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CommandBody), findsOneWidget);
      expect(find.byType(TimelineEntryShell), findsNothing);
      expect(find.text('/model'), findsOneWidget);
      expect(find.text('Switched to claude-sonnet'), findsOneWidget);
    });

    testWidgets('interaction renders its footer as the entry', (tester) async {
      await tester.pumpWidget(
        _view(
          _message(TimelineEntryType.interaction),
          footer: const Text('approval card'),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('approval card'), findsOneWidget);
      expect(find.byType(TimelineEntryShell), findsNothing);
    });

    testWidgets('message entries render nothing', (tester) async {
      await tester.pumpWidget(_view(_message(TimelineEntryType.message)));
      await tester.pumpAndSettle();

      expect(find.byType(TimelineEntryShell), findsNothing);
      expect(find.byType(CommandBody), findsNothing);
      expect(find.byType(Text), findsNothing);
    });
  });

  group('TimelineEntryView slots', () {
    testWidgets('footer reaches the shell and stays visible collapsed', (
      tester,
    ) async {
      await tester.pumpWidget(
        _view(
          _message(
            TimelineEntryType.toolCall,
            toolCalls: [
              ToolCallRecord(toolName: 'bash', status: ToolCallStatus.ok),
            ],
          ),
          footer: const Text('footer card'),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('footer card'), findsOneWidget);
      expect(
        find.descendant(
          of: find.byType(TimelineEntryShell),
          matching: find.text('footer card'),
        ),
        findsOneWidget,
      );
    });

    testWidgets('entryState reaches the shell', (tester) async {
      await tester.pumpWidget(
        _view(
          _message(
            TimelineEntryType.toolCall,
            toolCalls: [
              ToolCallRecord(toolName: 'bash', status: ToolCallStatus.ok),
            ],
          ),
          entryState: EntryState.stale,
        ),
      );
      await tester.pumpAndSettle();

      final opacity = tester.widget<Opacity>(
        find
            .descendant(
              of: find.byType(TimelineEntryShell),
              matching: find.byType(Opacity),
            )
            .first,
      );
      expect(opacity.opacity, 0.6);
    });
  });
}
