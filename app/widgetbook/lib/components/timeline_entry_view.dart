import 'dart:async';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

import '../knobs.dart';

ChatMessage _message(
  TimelineEntryType type, {
  bool streaming = false,
  Stream<String>? stream,
  String? summary,
}) {
  final message = ChatMessage(
    id: 'entry',
    role: 'assistant',
    content: '',
    isStreaming: streaming,
    timelineType: type,
    thinkingContent: type == TimelineEntryType.thinking
        ? 'The user wants a summary. I should read the file first.'
        : null,
    subagentTask: 'Research authentication',
    subagentSummary: summary,
    commandName: 'model',
    commandAckText: 'Switched to claude-sonnet',
    toolCalls: type == TimelineEntryType.toolCall
        ? [
            ToolCallRecord(
              toolName: 'web-search',
              status: ToolCallStatus.ok,
              arguments: const {'query': 'flutter widgets'},
              result: '3 results found',
            ),
          ]
        : const [],
  );
  message.thinkingTokenStream = stream;
  return message;
}

ChatMessage _subagentMessage() {
  final message = _message(TimelineEntryType.subagent);
  message.subagentThinking = 'Comparing OAuth providers...';
  message.subagentToolCalls = [
    ToolCallRecord(toolName: 'web-search', status: ToolCallStatus.ok),
    ToolCallRecord(toolName: 'file-read', status: ToolCallStatus.pending),
  ];
  return message;
}

Stream<String> _reasoningStream() async* {
  for (final word in 'Weighing two options.'.split(' ')) {
    await Future<void>.delayed(const Duration(milliseconds: 120));
    yield '$word ';
  }
}

Widget _approval() => Padding(
  padding: const EdgeInsets.only(top: 8),
  child: Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      FilledButton(onPressed: () {}, child: const Text('Allow')),
      const SizedBox(width: 8),
      OutlinedButton(onPressed: () {}, child: const Text('Deny')),
    ],
  ),
);

Widget _view(
  ChatMessage message, {
  required EntryState state,
  required TimelineDensity density,
  Widget? footer,
}) => ThreadDensityScope(
  density: density,
  child: TimelineEntryView(
    key: ValueKey('${message.id}-${state.name}-${density.name}'),
    message: message,
    entryState: state,
    footer: footer,
  ),
);

final timelineEntryViewComponent = WidgetbookComponent(
  name: 'TimelineEntryView',
  useCases: [
    WidgetbookUseCase(
      name: 'playground',
      builder: (context) => _view(
        switch (enumKnob(
          context,
          label: 'type',
          values: const [
            TimelineEntryType.thinking,
            TimelineEntryType.toolCall,
            TimelineEntryType.subagent,
            TimelineEntryType.command,
          ],
          initial: TimelineEntryType.toolCall,
        )) {
          TimelineEntryType.thinking => _message(
            TimelineEntryType.thinking,
            stream: _reasoningStream(),
          ),
          TimelineEntryType.toolCall => _message(TimelineEntryType.toolCall),
          TimelineEntryType.subagent => _subagentMessage(),
          TimelineEntryType.command => _message(TimelineEntryType.command),
          _ => _message(TimelineEntryType.message),
        },
        state: enumKnob(
          context,
          label: 'entryState',
          values: EntryState.values,
          initial: EntryState.complete,
        ),
        density: enumKnob(
          context,
          label: 'density',
          values: TimelineDensity.values,
          initial: TimelineDensity.normal,
        ),
        footer: context.knobs.boolean(label: 'footer') ? _approval() : null,
      ),
    ),
    WidgetbookUseCase(
      name: 'thinking',
      builder: (context) => _view(
        _message(TimelineEntryType.thinking),
        state: EntryState.complete,
        density: TimelineDensity.normal,
      ),
    ),
    WidgetbookUseCase(
      name: 'tool call with footer',
      builder: (context) => _view(
        _message(TimelineEntryType.toolCall),
        state: EntryState.awaitingInput,
        density: TimelineDensity.normal,
        footer: _approval(),
      ),
    ),
    WidgetbookUseCase(
      name: 'subagent',
      builder: (context) => _view(
        _subagentMessage(),
        state: EntryState.active,
        density: TimelineDensity.normal,
      ),
    ),
    WidgetbookUseCase(
      name: 'command',
      builder: (context) => _view(
        _message(TimelineEntryType.command),
        state: EntryState.complete,
        density: TimelineDensity.normal,
      ),
    ),
  ],
);
