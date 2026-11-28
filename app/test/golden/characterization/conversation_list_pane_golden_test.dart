@Tags(['golden'])
library;

import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/conversation_list.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../support/golden.dart';

// The app's ConversationListPane: a Riverpod adapter around the package's
// ConversationListView. The package goldens cover the pure view; this keeps
// the app-side composition (provider state, "New Chat", selection wiring).

class _FixedListNotifier extends ConversationListNotifier {
  _FixedListNotifier(this._state);

  final ConversationListState _state;

  @override
  Future<ConversationListState> build() async => _state;
}

Future<void> _pumpList(
  WidgetTester tester,
  List<ConversationListEntry> entries,
) async {
  await pumpGolden(
    tester,
    const SizedBox(width: 320, height: 420, child: ConversationListPane()),
    size: const Size(360, 460),
    overrides: [
      conversationListProvider.overrideWith(
        () => _FixedListNotifier(ConversationListState(conversations: entries)),
      ),
    ],
  );
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 100));
}

ConversationListEntry _entry(String id, String title, Duration ago) {
  final at = goldenAgo(ago);
  return ConversationListEntry(
    id: id,
    title: title,
    createdAt: at,
    updatedAt: at,
  );
}

void main() {
  testWidgets('empty', (tester) async {
    await _pumpList(tester, const []);
    await expectGolden('conversation_list_empty');
  });

  testWidgets('several items', (tester) async {
    await _pumpList(tester, [
      _entry('1', 'Plan the launch', const Duration(seconds: 20)),
      _entry(
        '2',
        'Debug flaky CI job',
        const Duration(minutes: 5, seconds: 30),
      ),
      _entry(
        '3',
        'A very long conversation title that has to be truncated',
        const Duration(hours: 2, minutes: 30),
      ),
      _entry(
        '4',
        'Summarise meeting notes',
        const Duration(days: 3, hours: 12),
      ),
    ]);
    await expectGolden('conversation_list_items');
  });
}
