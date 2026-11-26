/// A run of code text and the highlight.js scope that styles it.
class CodeToken {
  /// Creates a token; a null [scope] means plain, unstyled text.
  const CodeToken(this.text, {this.scope});

  /// The source text of this run.
  final String text;

  /// The highlight.js class name (`keyword`, `string`, ...), or null.
  final String? scope;

  @override
  bool operator ==(Object other) =>
      other is CodeToken && other.text == text && other.scope == scope;

  @override
  int get hashCode => Object.hash(text, scope);

  @override
  String toString() => 'CodeToken($scope: ${text.length} chars)';
}
