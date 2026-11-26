@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

ConversationListItem _item(
  String id,
  String title,
  Duration ago, {
  Duration? pinnedAgo,
}) => ConversationListItem(
  id: id,
  title: title,
  updatedAt: kGoldenNow.subtract(ago),
  pinnedAt: pinnedAgo == null ? null : kGoldenNow.subtract(pinnedAgo),
);

final _pinnedAndRecent = [
  _item(
    '1',
    'Release checklist',
    const Duration(days: 8),
    pinnedAgo: const Duration(days: 2),
  ),
  _item(
    '2',
    'Standing notes',
    const Duration(days: 20),
    pinnedAgo: const Duration(days: 6),
  ),
  _item('3', 'Plan the launch', const Duration(minutes: 5, seconds: 30)),
  _item('4', 'Debug flaky CI job', const Duration(hours: 2, minutes: 30)),
];

void main() {
  final cases = <String, List<ConversationListItem>>{
    'conversation_list_view_empty': const [],
    'conversation_list_view_items': [
      _item('1', 'Plan the launch', const Duration(seconds: 20)),
      _item('2', 'Debug flaky CI job', const Duration(minutes: 5, seconds: 30)),
      _item(
        '3',
        'A very long conversation title that has to be truncated',
        const Duration(hours: 2, minutes: 30),
      ),
      _item('4', 'Summarise meeting notes', const Duration(days: 3, hours: 12)),
    ],
    'conversation_list_view_pinned': _pinnedAndRecent,
    'conversation_list_view_selected': _pinnedAndRecent,
    'conversation_list_view_hover': _pinnedAndRecent,
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          SizedBox(
            width: 320,
            height: 420,
            child: ConversationListView(
              items: entry.value,
              selectedId: entry.key == 'conversation_list_view_selected'
                  ? '3'
                  : null,
              onSelect: (_) {},
              onRequestActions: (_) {},
              onDelete: (_) async => false,
              now: kGoldenNow,
              header: const Padding(
                padding: EdgeInsets.fromLTRB(16, 16, 16, 8),
                child: Text('Conversations'),
              ),
              emptyState: const Center(
                child: Text(
                  'No conversations yet.\nTap "New Chat" to start.',
                  textAlign: TextAlign.center,
                ),
              ),
            ),
          ),
          size: const Size(360, 460),
          brightness: brightness,
        );
        if (entry.key == 'conversation_list_view_hover') {
          final mouse = await tester.createGesture(
            kind: PointerDeviceKind.mouse,
          );
          addTearDown(mouse.removePointer);
          await mouse.addPointer(location: Offset.zero);
          await mouse.moveTo(tester.getCenter(find.text('Debug flaky CI job')));
        }
        await tester.pump(const Duration(milliseconds: 50));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
