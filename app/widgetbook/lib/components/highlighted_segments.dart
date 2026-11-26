import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

Widget _framed(Widget child) =>
    Padding(padding: const EdgeInsets.all(16), child: child);

final highlightedSegmentsComponent = WidgetbookComponent(
  name: 'HighlightedSegments',
  useCases: [
    WidgetbookUseCase(
      name: 'one hit',
      builder: (context) => _framed(
        const HighlightedSegments([
          SearchSegmentItem(text: 'The ', highlighted: false),
          SearchSegmentItem(text: 'Lisbon', highlighted: true),
          SearchSegmentItem(text: ' itinerary is ready', highlighted: false),
        ]),
      ),
    ),
    WidgetbookUseCase(
      name: 'two hits',
      builder: (context) => _framed(
        const HighlightedSegments([
          SearchSegmentItem(text: 'Book the ', highlighted: false),
          SearchSegmentItem(text: 'hotel', highlighted: true),
          SearchSegmentItem(text: ' in ', highlighted: false),
          SearchSegmentItem(text: 'Lisbon', highlighted: true),
        ]),
      ),
    ),
  ],
);
