import 'package:highlight/highlight.dart' show Node, highlight;
import 'package:highlight/languages/all.dart' show allLanguages;

import 'code_token.dart';

/// The seam between the memoising [CodeHighlighter] and a grammar engine.
///
/// This file is the only one in the package that imports `package:highlight`,
/// so swapping the engine is a one-file change.
abstract interface class SyntaxTokenizer {
  /// Whether [language] names a registered grammar.
  bool supports(String language);

  /// Tokenizes [code] with the grammar for [language].
  ///
  /// Callers check [supports] first.
  List<CodeToken> tokenize(String code, String language);
}

/// [SyntaxTokenizer] backed by the `highlight` package (highlight.js 10 era).
class HighlightJsTokenizer implements SyntaxTokenizer {
  /// Creates the default tokenizer.
  const HighlightJsTokenizer();

  /// Grammar names and aliases, lowercase.
  static final Set<String> _languages = {
    for (final entry in allLanguages.entries) ...{
      entry.key.toLowerCase(),
      ...?entry.value.aliases?.map((alias) => alias.toLowerCase()),
    },
  };

  @override
  bool supports(String language) => _languages.contains(language);

  @override
  List<CodeToken> tokenize(String code, String language) {
    final nodes = highlight.parse(code, language: language).nodes;
    final tokens = <CodeToken>[];
    void walk(Node node, String? inherited) {
      final scope = node.className ?? inherited;
      final value = node.value;
      if (value != null) {
        if (value.isEmpty) return;
        final last = tokens.isEmpty ? null : tokens.last;
        if (last != null && last.scope == scope) {
          tokens[tokens.length - 1] = CodeToken(
            last.text + value,
            scope: scope,
          );
        } else {
          tokens.add(CodeToken(value, scope: scope));
        }
      } else {
        for (final child in node.children ?? const <Node>[]) {
          walk(child, scope);
        }
      }
    }

    for (final node in nodes ?? const <Node>[]) {
      walk(node, null);
    }
    return tokens;
  }
}
