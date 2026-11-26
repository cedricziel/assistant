import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

SearchSegmentItem _plain(String text) =>
    SearchSegmentItem(text: text, highlighted: false);
SearchSegmentItem _hit(String text) =>
    SearchSegmentItem(text: text, highlighted: true);

Widget _framed(List<ConversationSearchItem> results) => SizedBox(
  width: 360,
  height: 240,
  child: ConversationSearchResultsView(results: results, onSelect: (_) {}),
);

final conversationSearchResultsViewComponent = WidgetbookComponent(
  name: 'ConversationSearchResultsView',
  useCases: [
    WidgetbookUseCase(
      name: 'title hit',
      builder: (context) => _framed([
        ConversationSearchItem(
          id: 'c1',
          title: 'Lisbon trip',
          titleSegments: [_hit('Lisbon'), _plain(' trip')],
        ),
      ]),
    ),
    WidgetbookUseCase(
      name: 'message hits',
      builder: (context) => _framed([
        ConversationSearchItem(
          id: 'c2',
          title: 'Weekend plans',
          titleSegments: [_plain('Weekend plans')],
          matches: [
            [
              _plain('…booked the '),
              _hit('hotel'),
              _plain(' near the station'),
            ],
            [_plain('Which '), _hit('hotel'), _plain(' has a late check-in?')],
          ],
        ),
        ConversationSearchItem(
          id: 'c3',
          title: 'Old hotel notes',
          titleSegments: [_plain('Old '), _hit('hotel'), _plain(' notes')],
          archivedAt: DateTime.utc(2026, 1, 2),
        ),
      ]),
    ),
    WidgetbookUseCase(name: 'empty', builder: (context) => _framed(const [])),
    WidgetbookUseCase(
      name: 'long snippet',
      builder: (context) => _framed([
        ConversationSearchItem(
          id: 'c4',
          title: 'Migration runbook',
          titleSegments: [_plain('Migration runbook')],
          matches: [
            [
              _plain(
                '…walk through every step of the migration, including the '
                'rollback plan, the ',
              ),
              _hit('monitoring'),
              _plain(
                ' dashboards to watch afterwards and who is on call for the '
                'first week…',
              ),
            ],
          ],
        ),
      ]),
    ),
  ],
);
