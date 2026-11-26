import 'code_token.dart';
import 'syntax_tokenizer.dart';

typedef _MemoKey = (String language, String code);

/// Highlights code through a [SyntaxTokenizer] with a bounded memo.
///
/// - No auto-detection: an absent or unknown language is plain text.
/// - Blocks over [maxCodeLength] characters are plain text.
/// - Plain results are never memoised.
/// - The memo is an LRU of [capacity] entries keyed by `(language, code)`
///   with full string equality.
/// - A growing streaming fence replaces its previous snapshot in place, so it
///   occupies one slot and does not evict finished blocks.
class CodeHighlighter {
  /// Creates a highlighter over [tokenizer].
  CodeHighlighter({this._tokenizer = const HighlightJsTokenizer()});

  /// Longest block that is highlighted.
  static const maxCodeLength = 20000;

  /// Maximum number of memoised blocks.
  static const capacity = 64;

  final SyntaxTokenizer _tokenizer;
  final _memo = <_MemoKey, List<CodeToken>>{};
  _MemoKey? _lastInserted;

  /// Number of memoised blocks.
  int get length => _memo.length;

  /// Tokens for [code] in [language], plain when it cannot be highlighted.
  List<CodeToken> highlight(String code, String? language) {
    final name = language?.trim().toLowerCase();
    if (name == null ||
        name.isEmpty ||
        code.length > maxCodeLength ||
        !_tokenizer.supports(name)) {
      return [CodeToken(code)];
    }

    final key = (name, code);
    final cached = _memo.remove(key);
    if (cached != null) {
      _memo[key] = cached;
      return cached;
    }

    final tokens = List<CodeToken>.unmodifiable(
      _tokenizer.tokenize(code, name),
    );
    final last = _lastInserted;
    if (last != null && last.$1 == name && code.startsWith(last.$2)) {
      _memo.remove(last);
    }
    _memo[key] = tokens;
    _lastInserted = key;
    if (_memo.length > capacity) _memo.remove(_memo.keys.first);
    return tokens;
  }
}
