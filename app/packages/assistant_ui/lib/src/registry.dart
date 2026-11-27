/// Every element the package exports, one per line and sorted, so parallel
/// PRs that add an element merge as trivial rebases.
///
/// To add an element, uncomment its slot line (and add a slot if missing).
/// Keep one blank line between entries so parallel PRs don't conflict.
///
/// `test/registry_test.dart` keeps it in step with the barrel and
/// `ELEMENTS.md`; the gallery's `coverage_test.dart` requires a use-case and a
/// `golden` test for each entry.
const List<String> kAssistantUiElements = [
  'ApprovalCard',
  'AttachmentTray',
  'BranchPicker',
  'ChatMarkdownScope',
  'ChatThreadLayout',
  'CodeBlock',
  'CodeDiffResultView',
  'CodeFileResultView',
  'CommandBody',
  'CompactExpandable',
  'Composer',
  'ComposerVoiceButton',
  'ConversationArchivedSection',
  'ConversationListView',
  'ConversationSearchResultsView',
  'DataTableResultView',
  'FeedbackButtons',
  'FeedbackCommentField',
  'GenericToolResultView',
  'HighlightedSegments',
  'JsonTreeView',
  'LinkPreviewResultView',
  'MessageActionButton',
  'MessageActions',
  'MessageBody',
  'MessageBubble',
  'MessageEditor',
  'MessageTimingIndicator',
  'PermissionRuleTile',
  'PlanResultView',
  'QuestionCard',
  'QueuedMessageBubble',
  'RegenerateMenuButton',
  'SlashCommandMenu',
  'SourcesResultView',
  'StarterPrompts',
  'SubagentBody',
  'SuggestionChips',
  'SurfaceFrame',
  'TerminalResultView',
  'ThinkingBody',
  'ThreadDensityScope',
  'ThreadEmptyState',
  'ThreadErrorBanner',
  'ThreadViewport',
  'TimelineEntryShell',
  'TimelineEntryView',
  'ToolCallBody',
  'ToolResultRendererScope',
  'ToolStatusIcon',
  'TurnProgressCard',
  'TypingIndicator',
  'UnknownInteractionCard',
  // 'BranchPicker', // slot: message-branching 9
  // 'FeedbackButtons', // slot: message-feedback 5
  // 'FeedbackCommentField', // slot: message-feedback 5
  // 'MessageEditor', // slot: message-branching 9
  // 'PermissionRuleTile', // slot: tool-approval-prompts 14b
  // 'TimelineEntryShell', // slot: chat-element-library 5b
  // 'UnknownInteractionCard', // slot: tool-approval-prompts 13b
];
