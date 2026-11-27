@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/golden.dart';

ChatMessage _message(
  TimelineEntryType type, {
  List<ToolCallRecord> toolCalls = const [],
  String? summary,
}) => ChatMessage(
  id: 'entry',
  role: 'assistant',
  content: '',
  timelineType: type,
  thinkingContent: type == TimelineEntryType.thinking
      ? 'Weighed two options and picked the second.'
      : null,
  toolCalls: toolCalls,
  subagentTask: 'Research golden testing',
  subagentSummary: summary,
  commandName: 'model',
  commandAckText: 'Switched to claude-sonnet',
);

Widget get _footer => Padding(
  padding: const EdgeInsets.only(top: 4),
  child: Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      FilledButton(onPressed: () {}, child: const Text('Allow')),
      const SizedBox(width: 8),
      OutlinedButton(onPressed: () {}, child: const Text('Deny')),
    ],
  ),
);

void main() {
  final cases = <String, ({Widget widget, Size size})>{
    'timeline_entry_view_thinking': (
      widget: TimelineEntryView(message: _message(TimelineEntryType.thinking)),
      size: const Size(420, 110),
    ),
    'timeline_entry_view_tool': (
      widget: TimelineEntryView(
        message: _message(
          TimelineEntryType.toolCall,
          toolCalls: [
            ToolCallRecord(
              toolName: 'web-search',
              status: ToolCallStatus.ok,
              arguments: const {'query': 'flutter golden tests'},
              result: '3 results found',
              startedAt: kGoldenNow,
              duration: const Duration(milliseconds: 1400),
            ),
          ],
        ),
      ),
      size: const Size(420, 110),
    ),
    'timeline_entry_view_tool_footer': (
      widget: TimelineEntryView(
        message: _message(
          TimelineEntryType.toolCall,
          toolCalls: [
            ToolCallRecord(toolName: 'bash', status: ToolCallStatus.pending),
          ],
        ),
        entryState: EntryState.awaitingInput,
        footer: _footer,
      ),
      size: const Size(420, 150),
    ),
    'timeline_entry_view_subagent': (
      widget: TimelineEntryView(
        message: _message(
          TimelineEntryType.subagent,
          summary: 'Found three prior approaches.',
        ),
      ),
      size: const Size(420, 110),
    ),
    'timeline_entry_view_command': (
      widget: TimelineEntryView(message: _message(TimelineEntryType.command)),
      size: const Size(420, 60),
    ),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          entry.value.widget,
          size: entry.value.size,
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 250));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
