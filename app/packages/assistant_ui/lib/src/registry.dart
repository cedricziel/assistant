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
  'ConversationListView',
  'ConversationSearchResultsView',
  'DataTableResultView',
  'FeedbackButtons',
  'FeedbackCommentField',
  'GenericToolResultView',
  'HighlightedSegments',
  'JsonTreeView',
  'MessageActionButton',
  'MessageActions',
  'MessageEditor',
  'MessageTimingIndicator',
  'PermissionRuleTile',
  'QuestionCard',
  'QueuedMessageBubble',
  'RegenerateMenuButton',
  'SlashCommandMenu',
  'StarterPrompts',
  'SubagentBody',
  'SuggestionChips',
  'SurfaceFrame',
  'ThinkingBody',
  'ThreadDensityScope',
  'ThreadEmptyState',
  'ThreadErrorBanner',
  'ThreadViewport',
  'TimelineEntryShell',
  'ToolCallBody',
  'ToolResultRendererScope',
  'ToolStatusIcon',
  'TurnProgressCard',
  'UnknownInteractionCard',
  // 'BranchPicker', // slot: message-branching 9
  // 'FeedbackButtons', // slot: message-feedback 5
  // 'FeedbackCommentField', // slot: message-feedback 5
  // 'LinkPreviewResultView', // slot: tool-result-renderers C5
  // 'MessageBody', // slot: chat-element-library 8b
  // 'MessageBubble', // slot: chat-element-library 8b
  // 'MessageEditor', // slot: message-branching 9
  // 'PermissionRuleTile', // slot: tool-approval-prompts 14b
  // 'PlanResultView', // slot: tool-result-renderers C5
  // 'SourcesResultView', // slot: tool-result-renderers C4
  // 'TerminalResultView', // slot: tool-result-renderers C4
  // 'TimelineEntryShell', // slot: chat-element-library 5b
  // 'TimelineEntryView', // slot: chat-element-library 7c
  // 'TypingIndicator', // slot: chat-element-library 8b
  // 'UnknownInteractionCard', // slot: tool-approval-prompts 13b
];
