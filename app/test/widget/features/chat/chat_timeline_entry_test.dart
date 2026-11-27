import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:assistant_api/assistant_api.dart'
    show MessageSummary, McpAppView, ToolCallSummary;
import 'package:built_collection/built_collection.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_timeline_entry.dart';
import 'package:assistant_app/features/chat/chat_timeline_reducers.dart';
import 'package:assistant_app/features/chat/mcp_app/mcp_app_provider.dart';
import 'package:assistant_ui/assistant_ui.dart';

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

// The app adapter puts the I/O-backed pieces the package cannot build around
// TimelineEntryView. These cases cover the MCP App frame; the entry kinds
// themselves are covered by the package's TimelineEntryView tests.

void main() {
  group('ChatTimelineEntry with an MCP App', () {
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
              child: ChatTimelineEntry(
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
