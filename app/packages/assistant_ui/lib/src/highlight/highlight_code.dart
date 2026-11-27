import 'code_token.dart';
import 'shared_code_highlighter.dart';

/// Highlights [code] as [language] with the shared memo.
///
/// Returns one plain token when [language] is null or unknown, or when the
/// block is too large to highlight. Never throws.
List<CodeToken> highlightCode(String code, String? language) =>
    sharedCodeHighlighter.highlight(code, language);
