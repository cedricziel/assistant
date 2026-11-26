/// Every element the package exports, one per line and sorted, so parallel
/// PRs that add an element merge as trivial rebases.
///
/// `test/registry_test.dart` keeps it in step with the barrel and
/// `ELEMENTS.md`; the gallery's `coverage_test.dart` requires a use-case and a
/// `golden` test for each entry.
const List<String> kAssistantUiElements = [
  'ApprovalCard',
  'CodeBlock',
  'MessageTimingIndicator',
  'StarterPrompts',
  'SuggestionChips',
  'ThreadDensityScope',
];
