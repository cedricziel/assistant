// -- Message branches --------------------------------------------------------

/// Where a message sits among its sibling versions.
///
/// Mirrors the server's branch metadata without depending on the API client.
class BranchInfo {
  const BranchInfo({
    required this.count,
    required this.index,
    required this.siblingIds,
  });

  /// How many versions there are (always at least 2).
  final int count;

  /// Zero-based position of the active version, in creation order.
  final int index;

  /// The first message of every version, in creation order. Switching to any
  /// of them selects that version.
  final List<String> siblingIds;
}
