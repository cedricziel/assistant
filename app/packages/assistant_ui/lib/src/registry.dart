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
  'BranchPicker',
  'CodeBlock',
  'MessageActionButton',
  'MessageActions',
  'MessageEditor',
  'MessageTimingIndicator',
  'PermissionRuleTile',
  'QueuedMessageBubble',
  'StarterPrompts',
  'SuggestionChips',
  'ThreadDensityScope',
  'TimelineEntryShell',
  'UnknownInteractionCard',
  // 'AttachmentTray', // slot: chat-element-library 11b
  // 'BranchPicker', // slot: message-branching 9
  // 'ChatThreadLayout', // slot: chat-element-library 10
  // 'CodeBlock', // slot: chat-a11y-and-code-blocks C4
  // 'CodeDiffResultView', // slot: tool-result-renderers C7
  // 'CodeFileResultView', // slot: tool-result-renderers C7
  // 'CommandBody', // slot: chat-element-library 7b
  // 'CompactExpandable', // slot: tool-result-renderers C2
  // 'Composer', // slot: chat-element-library 11a
  // 'ComposerVoiceButton', // slot: chat-element-library 11b
  // 'ConversationListView', // slot: chat-element-library 12
  // 'ConversationSearchResultsView', // slot: conversation-list-management 14
  // 'DataTableResultView', // slot: tool-result-renderers C6
  // 'FeedbackButtons', // slot: message-feedback 5
  // 'FeedbackCommentField', // slot: message-feedback 5
  // 'GenericToolResultView', // slot: chat-element-library 6
  // 'HighlightedSegments', // slot: conversation-list-management 14
  // 'JsonTreeView', // slot: tool-result-renderers C2b
  // 'LinkPreviewResultView', // slot: tool-result-renderers C5
  // 'MessageBody', // slot: chat-element-library 8b
  // 'MessageBubble', // slot: chat-element-library 8b
  // 'MessageEditor', // slot: message-branching 9
  // 'PermissionRuleTile', // slot: tool-approval-prompts 14b
  // 'PlanResultView', // slot: tool-result-renderers C5
  // 'QuestionCard', // slot: tool-approval-prompts 14a
  // 'RegenerateMenuButton', // slot: message-branching 10
  // 'SlashCommandMenu', // slot: chat-element-library 11a
  // 'SourcesResultView', // slot: tool-result-renderers C4
  // 'SubagentBody', // slot: chat-element-library 7b
  // 'SurfaceFrame', // slot: generative-ui-a2ui 13
  // 'TerminalResultView', // slot: tool-result-renderers C4
  // 'ThinkingBody', // slot: chat-element-library 7a
  // 'ThreadEmptyState', // slot: chat-element-library 9
  // 'ThreadErrorBanner', // slot: chat-element-library 9
  // 'ThreadViewport', // slot: chat-element-library 10
  // 'TimelineEntryShell', // slot: chat-element-library 5b
  // 'TimelineEntryView', // slot: chat-element-library 7c
  // 'ToolCallBody', // slot: chat-element-library 6
  // 'ToolStatusIcon', // slot: chat-element-library 6
  // 'TurnProgressCard', // slot: chat-element-library 9
  // 'TypingIndicator', // slot: chat-element-library 8b
  // 'UnknownInteractionCard', // slot: tool-approval-prompts 13b
];
