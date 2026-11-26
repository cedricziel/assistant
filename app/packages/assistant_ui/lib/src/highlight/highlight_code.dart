import 'code_highlighter.dart';
import 'code_token.dart';

final _shared = CodeHighlighter();

/// Highlights [code] as [language] with the shared memo.
///
/// Returns one plain token when [language] is null or unknown, or when the
/// block is too large to highlight. Never throws.
List<CodeToken> highlightCode(String code, String? language) =>
    _shared.highlight(code, language);
