import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/command_event_tile.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('CommandEventTile', () {
    testWidgets('renders command name and ack text', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: CommandEventTile(
              message: ChatMessage(
                id: 'cmd-1',
                role: 'system',
                content: 'Model set to gpt-4o',
                timelineType: TimelineEntryType.command,
                commandName: 'model',
                commandAckText: 'Model set to gpt-4o',
              ),
            ),
          ),
        ),
      );

      // Should show the command name with / prefix.
      expect(find.text('/model'), findsOneWidget);
      // Should show the ack text.
      expect(find.text('Model set to gpt-4o'), findsOneWidget);
    });

    testWidgets('uses system-event styling (not a chat bubble)', (
      tester,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: CommandEventTile(
              message: ChatMessage(
                id: 'cmd-2',
                role: 'system',
                content: 'New conversation started',
                timelineType: TimelineEntryType.command,
                commandName: 'new',
                commandAckText: 'New conversation started',
              ),
            ),
          ),
        ),
      );

      // Should have the command_event_tile key for identification.
      expect(find.byKey(const Key('command_event_tile')), findsOneWidget);
      // Should render as a centered row, not a bubble.
      expect(find.byType(Row), findsOneWidget);
    });

    testWidgets('renders with terminal icon', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: CommandEventTile(
              message: ChatMessage(
                id: 'cmd-3',
                role: 'system',
                content: 'ok',
                timelineType: TimelineEntryType.command,
                commandName: 'compact',
                commandAckText: 'History compacted',
              ),
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.terminal), findsOneWidget);
    });
  });
}
