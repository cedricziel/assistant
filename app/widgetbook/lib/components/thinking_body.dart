import 'dart:async';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:widgetbook/widgetbook.dart';

import '../knobs.dart';

const _reasoning =
    'The user wants a summary of the report. I should read the file first, '
    'then pull out the three main findings.';

/// Emits [_reasoning] a few words at a time, then closes.
Stream<String> _reasoningStream() async* {
  for (final word in _reasoning.split(' ')) {
    await Future<void>.delayed(const Duration(milliseconds: 120));
    yield '$word ';
  }
}

final thinkingBodyComponent = WidgetbookComponent(
  name: 'ThinkingBody',
  useCases: [
    WidgetbookUseCase(
      name: 'playground',
      builder: (context) => ThreadDensityScope(
        density: enumKnob(
          context,
          label: 'density',
          values: TimelineDensity.values,
          initial: TimelineDensity.normal,
        ),
        child: ThinkingBody(
          entryState: enumKnob(
            context,
            label: 'entry state',
            values: EntryState.values,
            initial: EntryState.complete,
          ),
          content: context.knobs.string(
            label: 'content',
            initialValue: _reasoning,
          ),
          elapsed: context.knobs.boolean(label: 'elapsed (4s)')
              ? const Duration(seconds: 4)
              : null,
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'streaming',
      builder: (context) => ThinkingBody(
        entryState: EntryState.active,
        tokenStream: _reasoningStream(),
      ),
    ),
    WidgetbookUseCase(
      name: 'elapsed',
      builder: (context) => const ThinkingBody(
        entryState: EntryState.complete,
        content: _reasoning,
        elapsed: Duration(seconds: 4),
      ),
    ),
    WidgetbookUseCase(
      name: 'empty',
      builder: (context) =>
          const ThinkingBody(entryState: EntryState.complete, content: ''),
    ),
  ],
);
