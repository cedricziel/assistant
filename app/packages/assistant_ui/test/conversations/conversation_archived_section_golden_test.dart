@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

ConversationListItem _item(String id, String title, Duration ago) =>
    ConversationListItem(
      id: id,
      title: title,
      updatedAt: kGoldenNow.subtract(ago),
    );

ConversationListItem _archived(String id, String title, Duration ago) =>
    ConversationListItem(
      id: id,
      title: title,
      updatedAt: kGoldenNow.subtract(ago),
      archivedAt: kGoldenNow.subtract(const Duration(days: 1)),
    );

void main() {
  final live = [
    _item('1', 'Plan the launch', const Duration(minutes: 5, seconds: 30)),
    _item('2', 'Debug flaky CI job', const Duration(hours: 2, minutes: 30)),
  ];
  final archived = [
    _archived('a1', 'Old trip planning', const Duration(days: 12)),
    _archived('a2', 'Resolved incident review', const Duration(days: 30)),
  ];

  final cases = <String, ({bool expanded, bool loading})>{
    'conversation_archived_collapsed': (expanded: false, loading: false),
    'conversation_archived_expanded': (expanded: true, loading: false),
    'conversation_archived_loading': (expanded: true, loading: true),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        final loading = entry.value.loading;
        await pumpGolden(
          tester,
          SizedBox(
            width: 320,
            height: 420,
            child: ConversationListView(
              items: live,
              onSelect: (_) {},
              onDelete: (_) async => false,
              now: kGoldenNow,
              header: const Padding(
                padding: EdgeInsets.fromLTRB(16, 16, 16, 8),
                child: Text('Conversations'),
              ),
              archivedSection: ConversationArchivedSection(
                expanded: entry.value.expanded,
                loading: loading,
                items: loading ? const [] : archived,
                onExpandedChanged: (_) {},
                onSelect: (_) {},
                onDelete: (_) async => false,
                onRequestActions: (_) {},
                now: kGoldenNow,
              ),
            ),
          ),
          size: const Size(360, 460),
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 50));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
