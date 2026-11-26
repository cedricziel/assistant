import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for ConversationSearchSummary
void main() {
  final instance = ConversationSearchSummaryBuilder();
  // TODO add properties to the builder and call build()

  group(ConversationSearchSummary, () {
    // DateTime createdAt
    test('to test the property `createdAt`', () async {
      // TODO
    });

    // String id
    test('to test the property `id`', () async {
      // TODO
    });

    // Up to 3 matching messages, best first; empty when only the title matched.
    // BuiltList<MessageSearchMatch> matches
    test('to test the property `matches`', () async {
      // TODO
    });

    // When the conversation was pinned; absent when it is not pinned.
    // DateTime pinnedAt
    test('to test the property `pinnedAt`', () async {
      // TODO
    });

    // String title
    test('to test the property `title`', () async {
      // TODO
    });

    // The title split into highlighted and plain runs.
    // BuiltList<SearchSegment> titleSegments
    test('to test the property `titleSegments`', () async {
      // TODO
    });

    // DateTime updatedAt
    test('to test the property `updatedAt`', () async {
      // TODO
    });
  });
}
