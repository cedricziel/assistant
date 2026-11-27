import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:assistant_app/features/chat/chat_timeline_entry.dart';

/// The parsed model of the fake contract used by these tests.
class _FakeResult {
  const _FakeResult(this.label);

  final String label;
}

/// A renderer for a contract no producer ships, standing in for the ones that
/// arrive in C4–C7.
class _FakeRenderer extends ToolResultRenderer<_FakeResult> {
  const _FakeRenderer();

  @override
  String get kind => 'fake';

  @override
  _FakeResult? parse(Map<String, Object?> data) {
    final label = data['label'];
    if (data['kind'] != kind || label is! String) return null;
    return _FakeResult(label);
  }

  @override
  Widget summary(BuildContext context, _FakeResult model) =>
      Text('summary ${model.label}');

  @override
  Widget build(BuildContext context, _FakeResult model, ToolCallStatus status) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text('full ${model.label}'),
        TextButton(
          onPressed: () =>
              ChatMarkdownScope.of(context)?.onTapLink?.call(model.label),
          child: const Text('open'),
        ),
      ],
    );
  }
}

const _registry = ToolResultRendererRegistry(byKind: [_FakeRenderer()]);

const _fakeData = {'kind': 'fake', 'label': 'https://example.com/source'};

Widget _wrap(
  Widget child, {
  void Function(String url)? onTapLink,
  TimelineDensity density = TimelineDensity.normal,
}) {
  return MaterialApp(
    home: MediaQuery(
      data: const MediaQueryData(size: Size(500, 800)),
      child: Scaffold(
        body: ChatMarkdownScope(
          onTapLink: onTapLink,
          child: ThreadDensityScope(
            density: density,
            child: ToolResultRendererScope(
              resolve: _registry.build,
              child: child,
            ),
          ),
        ),
      ),
    ),
  );
}

ChatMessage _toolCallEntry(ToolCallStatus status) {
  return ChatMessage(
    id: 'toolcall-1',
    role: 'assistant',
    content: '',
    timelineType: TimelineEntryType.toolCall,
    toolCalls: [
      ToolCallRecord(
        toolName: 'fake-tool',
        status: status,
        result: 'ok',
        data: _fakeData,
      ),
    ],
  );
}

ChatMessage _subagentEntry(ToolCallStatus status) {
  return ChatMessage(
      id: 'subagent-1',
      role: 'assistant',
      content: '',
      timelineType: TimelineEntryType.subagent,
      subagentId: 'sub-1',
      subagentTask: 'Research the topic',
    )
    ..subagentToolCalls.add(
      ToolCallRecord(
        toolName: 'fake-tool',
        status: status,
        result: 'ok',
        data: _fakeData,
      ),
    );
}

Future<void> _pumpEntry(
  WidgetTester tester,
  ChatMessage message, {
  void Function(String url)? onTapLink,
}) async {
  await tester.pumpWidget(
    _wrap(
      ChatTimelineEntry(message: message, entryState: EntryState.complete),
      onTapLink: onTapLink,
    ),
  );
  await tester.pump();
}

void main() {
  group('tool result renderer scope in the chat timeline', () {
    testWidgets('a completed ok call shows the renderer only after a tap', (
      tester,
    ) async {
      await _pumpEntry(tester, _toolCallEntry(ToolCallStatus.ok));

      expect(
        find.text('full https://example.com/source'),
        findsNothing,
        reason: 'a successful renderer body stays collapsed behind the header',
      );

      await tester.tap(find.byKey(TimelineEntryShell.headerKey));
      await tester.pumpAndSettle();

      expect(
        find.text('full https://example.com/source'),
        findsOneWidget,
        reason: 'the test registry resolves the fake kind through the scope',
      );
    });

    testWidgets('an error call opens without a tap', (tester) async {
      await _pumpEntry(tester, _toolCallEntry(ToolCallStatus.error));

      expect(
        find.text('full https://example.com/source'),
        findsOneWidget,
        reason: 'an error status force-expands the result view',
      );
    });

    testWidgets(
      'a link inside the result reaches ChatMarkdownScope.onTapLink',
      (tester) async {
        final tapped = <String>[];
        await _pumpEntry(
          tester,
          _toolCallEntry(ToolCallStatus.error),
          onTapLink: tapped.add,
        );

        await tester.tap(find.text('open'));
        await tester.pump();

        expect(tapped, ['https://example.com/source']);
      },
    );

    testWidgets('a subagent inner tool call stays a compact row', (
      tester,
    ) async {
      await tester.pumpWidget(
        _wrap(
          ChatTimelineEntry(
            message: _subagentEntry(ToolCallStatus.ok),
            entryState: EntryState.active,
          ),
        ),
      );
      await tester.pump();

      expect(
        find.text('fake-tool'),
        findsOneWidget,
        reason: 'the inner call renders as the compact row',
      );
      expect(
        find.text('full https://example.com/source'),
        findsNothing,
        reason: 'subagent timelines never consult the renderer scope',
      );
      expect(find.byType(GenericToolResultView), findsNothing);
    });
  });
}
