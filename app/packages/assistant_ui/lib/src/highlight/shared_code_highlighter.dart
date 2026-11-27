import 'code_highlighter.dart';

/// The highlighter behind `highlightCode`.
///
/// Tests replace it with one over a spy tokenizer to observe which blocks are
/// tokenised; production code never reassigns it.
CodeHighlighter sharedCodeHighlighter = CodeHighlighter();
