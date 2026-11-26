import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('segments compare by value', () {
    expect(
      const SearchSegmentItem(text: 'a', highlighted: true),
      const SearchSegmentItem(text: 'a', highlighted: true),
    );
    expect(
      const SearchSegmentItem(text: 'a', highlighted: true),
      isNot(const SearchSegmentItem(text: 'a', highlighted: false)),
    );
  });

  test('an item is archived when archivedAt is set', () {
    const live = ConversationSearchItem(
      id: 'a',
      title: 'A',
      titleSegments: [SearchSegmentItem(text: 'A', highlighted: false)],
    );
    final archived = ConversationSearchItem(
      id: 'b',
      title: 'B',
      titleSegments: const [SearchSegmentItem(text: 'B', highlighted: false)],
      archivedAt: DateTime.utc(2026),
    );

    expect(live.archivedAt, isNull);
    expect(archived.archivedAt, isNotNull);
  });
}
