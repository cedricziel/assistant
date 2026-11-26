import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_ui/src/highlight/code_highlighter.dart';
import 'package:assistant_ui/src/highlight/code_token.dart';
import 'package:assistant_ui/src/highlight/highlight_code.dart';
import 'package:assistant_ui/src/highlight/syntax_tokenizer.dart';

/// Records every call and tokenizes to one token per call.
class _SpyTokenizer implements SyntaxTokenizer {
  final known = const {'rust', 'dart'};
  final calls = <(String, String)>[];

  @override
  bool supports(String language) => known.contains(language);

  @override
  List<CodeToken> tokenize(String code, String language) {
    calls.add((language, code));
    return [CodeToken(code, scope: 'keyword')];
  }
}

void main() {
  group('highlightCode (highlight.js backend)', () {
    test('rust yields keyword tokens', () {
      final tokens = highlightCode('fn main() { let x = 1; }', 'rust');

      expect(tokens.any((t) => t.scope == 'keyword'), isTrue);
      expect(tokens.map((t) => t.text).join(), 'fn main() { let x = 1; }');
    });

    test('language lookup ignores case and surrounding whitespace', () {
      final tokens = highlightCode('let x = 1;', ' Rust ');

      expect(tokens.any((t) => t.scope == 'keyword'), isTrue);
    });

    test('unknown or absent language is one plain token', () {
      for (final language in <String?>['not-a-language', null, '']) {
        final tokens = highlightCode('let x = 1;', language);

        expect(tokens, hasLength(1), reason: 'language: $language');
        expect(tokens.single.text, 'let x = 1;');
        expect(tokens.single.scope, isNull);
      }
    });
  });

  group('CodeHighlighter', () {
    late _SpyTokenizer spy;
    late CodeHighlighter highlighter;

    setUp(() {
      spy = _SpyTokenizer();
      highlighter = CodeHighlighter(tokenizer: spy);
    });

    test('unknown language and null call nothing and memoise nothing', () {
      final unknown = highlighter.highlight('x', 'nope');
      final absent = highlighter.highlight('x', null);

      expect(unknown.single.scope, isNull);
      expect(absent.single.text, 'x');
      expect(spy.calls, isEmpty);
      expect(highlighter.length, 0);
    });

    test('20,001 characters skips the tokenizer and is not memoised', () {
      final tokens = highlighter.highlight('a' * 20001, 'dart');

      expect(tokens.single.scope, isNull);
      expect(spy.calls, isEmpty);
      expect(highlighter.length, 0);
    });

    test('20,000 characters is still highlighted', () {
      highlighter.highlight('a' * 20000, 'dart');

      expect(spy.calls, hasLength(1));
    });

    test('a repeated (language, code) is served from the memo', () {
      final first = highlighter.highlight('let x', 'rust');
      final second = highlighter.highlight('let x', 'rust');

      expect(second, same(first));
      expect(spy.calls, hasLength(1));
    });

    test('the memo is keyed by full equality, not length', () {
      highlighter.highlight('aaaa', 'rust');
      highlighter.highlight('bbbb', 'rust');
      highlighter.highlight('aaaa', 'dart');

      expect(spy.calls, hasLength(3));
      expect(highlighter.length, 3);
    });

    test('evicts the least recently used entry beyond 64 entries', () {
      for (var i = 0; i < 64; i++) {
        highlighter.highlight('block $i', 'rust');
      }
      highlighter.highlight('block 0', 'rust'); // touch: now most recent
      highlighter.highlight('block 64', 'rust'); // evicts block 1
      expect(highlighter.length, 64);

      spy.calls.clear();
      highlighter.highlight('block 0', 'rust');
      expect(spy.calls, isEmpty, reason: 'recently used entry survives');
      highlighter.highlight('block 1', 'rust');
      expect(spy.calls, hasLength(1), reason: 'oldest entry was evicted');
    });

    test('a growing fence replaces itself and spares a finished fence', () {
      const finished = 'void main() {}';
      highlighter.highlight(finished, 'dart');

      var growing = '';
      for (var i = 0; i < 100; i++) {
        growing += 'let a$i = $i;\n';
        highlighter.highlight(growing, 'rust');
        expect(highlighter.length, lessThanOrEqualTo(2));
      }
      expect(highlighter.length, 2);

      spy.calls.clear();
      highlighter.highlight(finished, 'dart');
      expect(spy.calls, isEmpty, reason: 'finished fence is still a hit');
    });

    test('a different language does not replace the last entry', () {
      highlighter.highlight('fn', 'rust');
      highlighter.highlight('fn main', 'dart');

      expect(highlighter.length, 2);
    });
  });
}
