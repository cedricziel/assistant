import 'dart:async';

import 'package:material_ui/material_ui.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart'
    hide ToolCallStatus;
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_api/assistant_api.dart'
    show MessageSummary, McpAppView, ToolCallSummary;
import 'package:built_collection/built_collection.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_timeline_reducers.dart';
import 'package:assistant_app/features/chat/mcp_app/mcp_app_provider.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_app/features/chat/streaming_timeline_entry.dart';

import '../../../support/fake_mcp_app.dart';

/// Wrap widget under test in a MaterialApp with the given screen width.
Widget _wrap(Widget child, {double width = 500}) {
  return MaterialApp(
    home: MediaQuery(
      data: MediaQueryData(size: Size(width, 800)),
      child: Scaffold(body: child),
    ),
  );
}

/// Renders a [StreamingTimelineEntry] under a [ThreadDensityScope].
Widget _entry({
  required ChatMessage message,
  required TimelineDensity density,
  EntryState entryState = EntryState.complete,
}) => ThreadDensityScope(
  density: density,
  child: StreamingTimelineEntry(message: message, entryState: entryState),
);

ChatMessage _thinkingMessage({
  bool isStreaming = false,
  String? thinkingContent,
}) {
  return ChatMessage(
    id: 'test-thinking',
    role: 'assistant',
    content: '',
    isStreaming: isStreaming,
    timelineType: TimelineEntryType.thinking,
    thinkingContent: thinkingContent,
  );
}

void main() {
  group('StreamingTimelineEntry', () {
    testWidgets('density changes based on MediaQuery width', (tester) async {
      final msg = ChatMessage(
        id: 'test-tool',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.toolCall,
        toolCalls: [
          ToolCallRecord(
            toolName: 'file-read',
            status: ToolCallStatus.ok,
            arguments: {'path': '/tmp/test.txt'},
            result: 'file contents here',
          ),
        ],
      );

      // Compact width (< 400): should not show status icon by default.
      await tester.pumpWidget(
        _wrap(
          _entry(
            message: msg,
            density: TimelineDensity.compact,
            entryState: EntryState.complete,
          ),
          width: 350,
        ),
      );
      await tester.pumpAndSettle();

      // Tool name should still be visible.
      expect(find.text('file-read'), findsOneWidget);

      // Normal width: shows tool name + status.
      await tester.pumpWidget(
        _wrap(
          _entry(
            message: msg,
            density: TimelineDensity.normal,
            entryState: EntryState.complete,
          ),
          width: 500,
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('file-read'), findsOneWidget);
      expect(find.byIcon(Icons.check_circle_outline), findsOneWidget);

      // Expanded width: same visible elements.
      await tester.pumpWidget(
        _wrap(
          _entry(
            message: msg,
            density: TimelineDensity.expanded,
            entryState: EntryState.complete,
          ),
          width: 800,
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('file-read'), findsOneWidget);
      expect(find.byIcon(Icons.check_circle_outline), findsOneWidget);
    });
  });

  group('Thinking entry', () {
    testWidgets('renders StreamMarkdown when active with token stream', (
      tester,
    ) async {
      final controller = StreamController<String>.broadcast();
      addTearDown(controller.close);

      final msg = ChatMessage(
        id: 'test-thinking-stream',
        role: 'assistant',
        content: '',
        isStreaming: true,
        timelineType: TimelineEntryType.thinking,
        thinkingContent: '',
      );
      msg.thinkingTokenStream = controller.stream;

      await tester.pumpWidget(
        _wrap(
          _entry(
            message: msg,
            density: TimelineDensity.normal,
            entryState: EntryState.active,
          ),
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      // Active entry should auto-expand and show StreamMarkdown.
      expect(find.byType(StreamMarkdown), findsOneWidget);
      expect(find.text('Thinking...'), findsOneWidget);
      // The spinner should be present.
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
    });

    testWidgets('collapses to summary when complete', (tester) async {
      final msg = _thinkingMessage(
        isStreaming: false,
        thinkingContent: 'I analyzed the problem thoroughly.',
      );

      await tester.pumpWidget(
        _wrap(
          _entry(
            message: msg,
            density: TimelineDensity.normal,
            entryState: EntryState.complete,
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Complete entry shows "Thought" label (not "Thinking...").
      expect(find.text('Thought'), findsOneWidget);
      expect(find.text('Thinking...'), findsNothing);

      // Complete entry should be collapsed (expand chevron visible).
      expect(find.byIcon(Icons.expand_more_rounded), findsOneWidget);

      // No spinner when complete.
      expect(find.byType(CircularProgressIndicator), findsNothing);
    });

    testWidgets('compact density starts collapsed even when active', (
      tester,
    ) async {
      final controller = StreamController<String>.broadcast();
      addTearDown(controller.close);

      final msg = ChatMessage(
        id: 'test-thinking-compact',
        role: 'assistant',
        content: '',
        isStreaming: true,
        timelineType: TimelineEntryType.thinking,
        thinkingContent: 'Some content',
      );
      msg.thinkingTokenStream = controller.stream;

      await tester.pumpWidget(
        _wrap(
          _entry(
            message: msg,
            density: TimelineDensity.compact,
            entryState: EntryState.active,
          ),
          width: 350,
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      // Compact density does NOT auto-expand, shows expand chevron.
      expect(find.byIcon(Icons.expand_more_rounded), findsOneWidget);
      // StreamMarkdown should not be visible (collapsed).
      expect(find.byType(StreamMarkdown), findsNothing);
    });
  });

  group('Subagent entry', () {
    testWidgets('renders nested tool call chips and thinking when active', (
      tester,
    ) async {
      final msg = ChatMessage(
        id: 'test-subagent-active',
        role: 'assistant',
        content: '',
        isStreaming: true,
        timelineType: TimelineEntryType.subagent,
        subagentId: 'agent-1',
        subagentTask: 'Research authentication',
      );
      msg.subagentThinking = 'Analyzing OAuth providers...';
      msg.subagentToolCalls = [
        ToolCallRecord(toolName: 'web-search', status: ToolCallStatus.ok),
        ToolCallRecord(toolName: 'file-read', status: ToolCallStatus.pending),
      ];

      await tester.pumpWidget(
        _wrap(
          _entry(
            message: msg,
            density: TimelineDensity.expanded,
            entryState: EntryState.active,
          ),
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      // Subagent task should appear as the label.
      expect(find.text('Research authentication'), findsOneWidget);
      // Inner thinking should be shown.
      expect(find.text('Analyzing OAuth providers...'), findsOneWidget);
      // Inner tool calls should render.
      expect(find.text('web-search'), findsOneWidget);
      expect(find.text('file-read'), findsOneWidget);
    });

    testWidgets('collapses to summary on completion', (tester) async {
      final msg = ChatMessage(
        id: 'test-subagent-complete',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.subagent,
        subagentId: 'agent-2',
        subagentTask: 'Research caching',
        subagentSummary:
            'Found 3 caching strategies applicable to our use case.',
      );

      await tester.pumpWidget(
        _wrap(
          _entry(
            message: msg,
            density: TimelineDensity.normal,
            entryState: EntryState.complete,
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Task label visible.
      expect(find.text('Research caching'), findsOneWidget);
      // Check icon for completion.
      expect(find.byIcon(Icons.check_circle_outline), findsOneWidget);
      // Summary is expandable content — entry starts collapsed on complete.
      expect(find.byIcon(Icons.expand_more_rounded), findsOneWidget);
    });
  });

  group('TimelineDensity', () {
    test('fromWidth returns compact for narrow screens', () {
      expect(TimelineDensity.fromWidth(350), TimelineDensity.compact);
    });

    test('fromWidth returns normal for medium screens', () {
      expect(TimelineDensity.fromWidth(500), TimelineDensity.normal);
    });

    test('fromWidth returns expanded for wide screens', () {
      expect(TimelineDensity.fromWidth(800), TimelineDensity.expanded);
    });

    test('fromWidth boundary: 400 is normal', () {
      expect(TimelineDensity.fromWidth(400), TimelineDensity.normal);
    });

    test('fromWidth boundary: 700 is normal', () {
      expect(TimelineDensity.fromWidth(700), TimelineDensity.normal);
    });

    test('fromWidth boundary: 701 is expanded', () {
      expect(TimelineDensity.fromWidth(701), TimelineDensity.expanded);
    });
  });

  group('StreamingTimelineEntry with an MCP App', () {
    Future<void> pumpEntry(WidgetTester tester, ChatMessage message) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            mcpAppSurfaceFactoryProvider.overrideWithValue(
              FakeMcpAppSurface.factory,
            ),
            mcpAppBackendProvider.overrideWith((ref, _) => FakeMcpAppBackend()),
          ],
          child: _wrap(
            ThreadDensityScope(
              density: TimelineDensity.normal,
              child: StreamingTimelineEntry(
                message: message,
                conversationId: 'conv-1',
              ),
            ),
          ),
        ),
      );
      await tester.pump();
    }

    Future<void> unmount(WidgetTester tester) async {
      await tester.pumpWidget(const SizedBox());
      await tester.pump(const Duration(seconds: 1));
    }

    void expectFrameBelowChip(WidgetTester tester) {
      final chip = find.text('mcp__weather__forecast');
      final frame = find.byKey(fakeMcpAppViewKey);
      expect(chip, findsOneWidget);
      expect(frame, findsOneWidget);
      expect(find.text('Third-party app from weather'), findsOneWidget);
      expect(
        tester.getTopLeft(frame).dy,
        greaterThan(tester.getBottomLeft(chip).dy),
        reason: 'the app renders below its chip',
      );
    }

    testWidgets('renders the app of a live tool result below the chip', (
      tester,
    ) async {
      final msgs = <ChatMessage>[];
      applyStatusEvent(
        msgs,
        const StatusEvent(
          'Calling tool: mcp__weather__forecast',
          toolCallId: 'call-1',
        ),
      );
      applyToolResultEvent(
        msgs,
        ToolResultEvent.fromJson({
          'tool_name': 'mcp__weather__forecast',
          'status': 'ok',
          'tool_call_id': 'call-1',
          'result': 'Sunny',
          'app': {
            'server': 'weather',
            'resource_uri': 'ui://weather/forecast',
            'html': '<p>Sunny</p>',
          },
        }),
      );

      await pumpEntry(tester, msgs.single);

      expectFrameBelowChip(tester);
      await unmount(tester);
    });

    testWidgets('renders the app again when the conversation is reloaded', (
      tester,
    ) async {
      final history = chatMessagesFromHistory([
        MessageSummary(
          (b) => b
            ..id = 'a1'
            ..role = 'assistant'
            ..content = ''
            ..turn = 1
            ..createdAt = DateTime.parse('2026-05-16T00:00:00Z')
            ..ttsAvailable = false
            ..toolCalls = ListBuilder<ToolCallSummary>([
              ToolCallSummary(
                (t) => t
                  ..name = 'mcp__weather__forecast'
                  ..status = 'ok'
                  ..toolCallId = 'call-1'
                  ..app.replace(
                    McpAppView(
                      (a) => a
                        ..server = 'weather'
                        ..resourceUri = 'ui://weather/forecast'
                        ..html = '<p>Sunny</p>'
                        ..permissions.camera = false
                        ..permissions.microphone = false
                        ..permissions.geolocation = false
                        ..permissions.clipboardWrite = false,
                    ),
                  ),
              ),
            ]),
        ),
      ]);

      await pumpEntry(tester, history.single);

      expectFrameBelowChip(tester);
      await unmount(tester);
    });

    testWidgets('renders no frame for a tool call without an app', (
      tester,
    ) async {
      FakeMcpAppSurface.created.clear();
      await pumpEntry(
        tester,
        ChatMessage(
          id: 'tc',
          role: 'assistant',
          content: '',
          timelineType: TimelineEntryType.toolCall,
          toolCalls: [
            ToolCallRecord(toolName: 'bash', status: ToolCallStatus.ok),
          ],
        ),
      );

      expect(find.byKey(fakeMcpAppViewKey), findsNothing);
      expect(FakeMcpAppSurface.created, isEmpty);
    });
  });
}
