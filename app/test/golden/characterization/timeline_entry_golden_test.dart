@Tags(['golden'])
library;

import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../support/golden.dart';

// Before-image of the timeline entry widgets (chat-element-library PR 1b),
// now rendered through TimelineEntryView. The extraction PRs must keep every
// image byte for byte.

const _size = Size(420, 240);

ToolCallRecord _tool(
  ToolCallStatus status, {
  Duration? duration = const Duration(milliseconds: 1400),
}) => ToolCallRecord(
  toolName: 'web-search',
  status: status,
  arguments: const {'query': 'flutter golden tests', 'limit': 3},
  result: switch (status) {
    ToolCallStatus.pending => null,
    ToolCallStatus.ok => '3 results found',
    ToolCallStatus.error => 'Request failed: connection refused',
    ToolCallStatus.denied => 'Denied by permission policy',
    ToolCallStatus.cancelled => 'Cancelled by user',
  },
  startedAt: kGoldenNow,
  duration: status == ToolCallStatus.pending ? null : duration,
);

ChatMessage _toolMessage(ToolCallStatus status) => ChatMessage(
  id: 'tool-${status.name}',
  role: 'assistant',
  content: '',
  timelineType: TimelineEntryType.toolCall,
  toolCalls: [_tool(status)],
);

ChatMessage _thinking({String? content, Stream<String>? stream}) {
  final msg = ChatMessage(
    id: 'thinking',
    role: 'assistant',
    content: '',
    timelineType: TimelineEntryType.thinking,
    thinkingContent: content,
  );
  msg.thinkingTokenStream = stream;
  return msg;
}

ChatMessage _subagent({
  String? summary,
  int toolCalls = 1,
  bool thinking = true,
}) {
  final msg = ChatMessage(
    id: 'sub',
    role: 'assistant',
    content: '',
    timelineType: TimelineEntryType.subagent,
    subagentId: 'researcher',
    subagentTask: 'Research golden testing',
    subagentSummary: summary,
  );
  if (thinking) msg.subagentThinking = 'Looking for prior art on goldens';
  msg.subagentToolCalls = [
    for (var i = 0; i < toolCalls; i++)
      ToolCallRecord(
        toolName: 'tool-$i',
        status: i == toolCalls - 1 ? ToolCallStatus.pending : ToolCallStatus.ok,
        startedAt: kGoldenNow,
      ),
  ];
  return msg;
}

ChatMessage _command() => ChatMessage(
  id: 'cmd',
  role: 'assistant',
  content: '',
  timelineType: TimelineEntryType.command,
  commandName: 'model',
  commandAckText: 'Switched to claude-sonnet',
);

Future<void> _golden(
  WidgetTester tester,
  String name,
  ChatMessage message, {
  required TimelineDensity density,
  EntryState state = EntryState.complete,
  bool expand = false,
  Brightness brightness = Brightness.light,
}) async {
  await pumpGolden(
    tester,
    ThreadDensityScope(
      density: density,
      child: TimelineEntryView(message: message, entryState: state),
    ),
    size: _size,
    brightness: brightness,
    alignment: Alignment.topLeft,
  );
  await tester.pump(const Duration(milliseconds: 50));
  if (expand) {
    await tester.tap(find.byIcon(Icons.expand_more_rounded).first);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 250));
  }
  await expectGolden(name);
}

void main() {
  group('thinking', () {
    for (final density in TimelineDensity.values) {
      testWidgets('active with stream, ${density.name}', (tester) async {
        final message = _thinking(
          stream: Stream.fromIterable(['Weighing **two** options. ']),
        );
        await _golden(
          tester,
          'timeline_thinking_active_stream_${density.name}',
          message,
          density: density,
          state: EntryState.active,
        );
      });

      testWidgets('complete, ${density.name}', (tester) async {
        await _golden(
          tester,
          'timeline_thinking_complete_${density.name}',
          _thinking(content: 'Weighed two options and picked the second.'),
          density: density,
          expand: true,
        );
      });
    }

    testWidgets('stale', (tester) async {
      await _golden(
        tester,
        'timeline_thinking_stale',
        _thinking(content: 'Weighed two options and picked the second.'),
        density: TimelineDensity.normal,
        state: EntryState.stale,
      );
    });
  });

  group('tool call', () {
    for (final status in ToolCallStatus.values) {
      for (final density in TimelineDensity.values) {
        final active = status == ToolCallStatus.pending
            ? EntryState.active
            : EntryState.complete;

        // An active entry outside compact density auto-expands and an error
        // entry is forced open, so neither has a collapsed image.
        final canCollapse =
            status != ToolCallStatus.error &&
            !(active == EntryState.active &&
                density != TimelineDensity.compact);

        if (canCollapse) {
          testWidgets('${status.name} collapsed, ${density.name}', (
            tester,
          ) async {
            await _golden(
              tester,
              'timeline_tool_${status.name}_collapsed_${density.name}',
              _toolMessage(status),
              density: density,
              state: active,
            );
          });
        }

        testWidgets('${status.name} expanded, ${density.name}', (tester) async {
          await _golden(
            tester,
            'timeline_tool_${status.name}_expanded_${density.name}',
            _toolMessage(status),
            density: density,
            state: active,
            // An active entry auto-expands outside compact density, and an
            // error entry is always expanded; tap only to open the rest.
            expand:
                !(status == ToolCallStatus.error) &&
                !(active == EntryState.active &&
                    density != TimelineDensity.compact),
          );
        });
      }
    }

    testWidgets('ok stale', (tester) async {
      await _golden(
        tester,
        'timeline_tool_ok_stale',
        _toolMessage(ToolCallStatus.ok),
        density: TimelineDensity.normal,
        state: EntryState.stale,
      );
    });

    testWidgets('ok dark', (tester) async {
      await _golden(
        tester,
        'timeline_tool_ok_expanded_dark',
        _toolMessage(ToolCallStatus.ok),
        density: TimelineDensity.normal,
        expand: true,
        brightness: Brightness.dark,
      );
    });
  });

  group('subagent', () {
    for (final density in TimelineDensity.values) {
      testWidgets('running, ${density.name}', (tester) async {
        await _golden(
          tester,
          'timeline_subagent_running_${density.name}',
          _subagent(),
          density: density,
          state: EntryState.active,
        );
      });

      testWidgets('complete with summary, ${density.name}', (tester) async {
        await _golden(
          tester,
          'timeline_subagent_complete_${density.name}',
          _subagent(summary: 'Found three prior approaches.'),
          density: density,
          expand: true,
        );
      });

      testWidgets('more than 2 inner calls, ${density.name}', (tester) async {
        await _golden(
          tester,
          'timeline_subagent_many_calls_${density.name}',
          _subagent(toolCalls: 4),
          density: density,
          state: EntryState.active,
        );
      });
    }

    testWidgets('stale after completion', (tester) async {
      await _golden(
        tester,
        'timeline_subagent_stale',
        _subagent(summary: 'Found three prior approaches.'),
        density: TimelineDensity.normal,
        state: EntryState.stale,
      );
    });
  });

  // TimelineEntryView draws a command entry with CommandBody, without a shell.
  group('command', () {
    for (final density in TimelineDensity.values) {
      testWidgets('tile, ${density.name}', (tester) async {
        final width = switch (density) {
          TimelineDensity.compact => 360.0,
          TimelineDensity.normal => 500.0,
          TimelineDensity.expanded => 760.0,
        };
        await pumpGolden(
          tester,
          TimelineEntryView(message: _command()),
          size: Size(width, 60),
        );
        await tester.pump(const Duration(milliseconds: 50));
        await expectGolden('timeline_command_${density.name}');
      });
    }
  });
}
