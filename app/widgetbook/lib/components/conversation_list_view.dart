import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

final _now = DateTime.utc(2026, 1, 15, 12);

ConversationListItem _item(String id, String title, Duration ago) =>
    ConversationListItem(id: id, title: title, updatedAt: _now.subtract(ago));

Widget _framed(List<ConversationListItem> items) => SizedBox(
  width: 320,
  height: 420,
  child: ConversationListView(
    items: items,
    onSelect: (_) {},
    onDelete: (_) async => false,
    now: _now,
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
);

final conversationListViewComponent = WidgetbookComponent(
  name: 'ConversationListView',
  useCases: [
    WidgetbookUseCase(name: 'empty', builder: (context) => _framed(const [])),
    WidgetbookUseCase(
      name: 'several items',
      builder: (context) => _framed([
        _item('1', 'Plan the launch', const Duration(seconds: 20)),
        _item(
          '2',
          'Debug flaky CI job',
          const Duration(minutes: 5, seconds: 30),
        ),
        _item(
          '3',
          'A very long conversation title that has to be truncated',
          const Duration(hours: 2, minutes: 30),
        ),
        _item(
          '4',
          'Summarise meeting notes',
          const Duration(days: 3, hours: 12),
        ),
      ]),
    ),
  ],
);
