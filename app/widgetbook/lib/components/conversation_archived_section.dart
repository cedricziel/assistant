import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

final _now = DateTime.utc(2026, 1, 15, 12);

ConversationListItem _archived(String id, String title, Duration ago) =>
    ConversationListItem(
      id: id,
      title: title,
      updatedAt: _now.subtract(ago),
      archivedAt: _now.subtract(const Duration(days: 1)),
    );

Widget _framed({
  bool expanded = true,
  List<ConversationListItem> items = const [],
  bool loading = false,
  String? error,
}) => SizedBox(
  width: 320,
  child: ConversationArchivedSection(
    expanded: expanded,
    onExpandedChanged: (_) {},
    items: items,
    loading: loading,
    error: error,
    onRetry: () {},
    onSelect: (_) {},
    onDelete: (_) async => false,
    onRequestActions: (_) {},
    now: _now,
  ),
);

final conversationArchivedSectionComponent = WidgetbookComponent(
  name: 'ConversationArchivedSection',
  useCases: [
    WidgetbookUseCase(
      name: 'collapsed',
      builder: (context) => _framed(expanded: false),
    ),
    WidgetbookUseCase(
      name: 'expanded',
      builder: (context) => _framed(
        items: [
          _archived('a1', 'Old trip planning', const Duration(days: 12)),
          _archived('a2', 'Resolved incident review', const Duration(days: 30)),
        ],
      ),
    ),
    WidgetbookUseCase(
      name: 'loading',
      builder: (context) => _framed(loading: true),
    ),
    WidgetbookUseCase(
      name: 'error',
      builder: (context) =>
          _framed(error: 'Failed to load archived conversations'),
    ),
    WidgetbookUseCase(name: 'empty', builder: (context) => _framed()),
  ],
);
