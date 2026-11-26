// -- Conversation search ------------------------------------------------------

/// A run of text in a search result, either matched by the query or not.
///
/// Concatenating the `text` of a list of segments yields the displayed string.
class SearchSegmentItem {
  const SearchSegmentItem({required this.text, required this.highlighted});

  final String text;

  /// Whether the query matched this run.
  final bool highlighted;

  @override
  bool operator ==(Object other) =>
      other is SearchSegmentItem &&
      other.text == text &&
      other.highlighted == highlighted;

  @override
  int get hashCode => Object.hash(text, highlighted);

  @override
  String toString() => 'SearchSegmentItem($text, highlighted: $highlighted)';
}

/// One conversation in a search result list.
class ConversationSearchItem {
  const ConversationSearchItem({
    required this.id,
    required this.title,
    required this.titleSegments,
    this.matches = const [],
    this.archivedAt,
  });

  final String id;

  /// The plain title, used when [titleSegments] is empty.
  final String title;

  /// The title with the matched parts marked.
  final List<SearchSegmentItem> titleSegments;

  /// Message snippets, best first. Empty for a title-only hit. A view shows at
  /// most three.
  final List<List<SearchSegmentItem>> matches;

  /// Set when the conversation is archived.
  final DateTime? archivedAt;
}
