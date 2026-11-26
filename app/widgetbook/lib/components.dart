import 'package:widgetbook/widgetbook.dart';

import 'components/approval_card.dart';
import 'components/branch_picker.dart';
import 'components/code_block.dart';
import 'components/compact_expandable.dart';
import 'components/conversation_list_view.dart';
import 'components/conversation_search_results_view.dart';
import 'components/feedback_buttons.dart';
import 'components/feedback_comment_field.dart';
import 'components/generic_tool_result_view.dart';
import 'components/highlighted_segments.dart';
import 'components/json_tree_view.dart';
import 'components/message_action_button.dart';
import 'components/message_actions.dart';
import 'components/message_editor.dart';
import 'components/message_timing_indicator.dart';
import 'components/permission_rule_tile.dart';
import 'components/question_card.dart';
import 'components/queued_message_bubble.dart';
import 'components/regenerate_menu_button.dart';
import 'components/starter_prompts.dart';
import 'components/suggestion_chips.dart';
import 'components/surface_frame.dart';
import 'components/thread_density_scope.dart';
import 'components/thread_empty_state.dart';
import 'components/thread_error_banner.dart';
import 'components/timeline_entry_shell.dart';
import 'components/tool_call_body.dart';
import 'components/tool_result_renderer_scope.dart';
import 'components/tool_status_icon.dart';
import 'components/turn_progress_card.dart';
import 'components/unknown_interaction_card.dart';

// import 'components/thread_viewport.dart'; // slot: chat-element-library 10

// import 'components/timeline_entry_shell.dart'; // slot: chat-element-library 5b

// import 'components/timeline_entry_view.dart'; // slot: chat-element-library 7c

// import 'components/typing_indicator.dart'; // slot: chat-element-library 8b

// import 'components/unknown_interaction_card.dart'; // slot: tool-approval-prompts 13b

/// One component per `kAssistantUiElements` entry, named after the element and
/// sorted by name.
///
/// To add an element, uncomment its slot line (and add a slot if missing).
/// Keep one blank line between entries so parallel PRs don't conflict.
final List<WidgetbookComponent> kComponents = [
  // attachmentTrayComponent, // slot: chat-element-library 11b
  // branchPickerComponent, // slot: message-branching 9
  // chatThreadLayoutComponent, // slot: chat-element-library 10
  // codeDiffResultViewComponent, // slot: tool-result-renderers C7
  // codeFileResultViewComponent, // slot: tool-result-renderers C7
  // commandBodyComponent, // slot: chat-element-library 7b
  // composerComponent, // slot: chat-element-library 11a
  // composerVoiceButtonComponent, // slot: chat-element-library 11b
  // dataTableResultViewComponent, // slot: tool-result-renderers C6
  // feedbackButtonsComponent, // slot: message-feedback 5
  // feedbackCommentFieldComponent, // slot: message-feedback 5
  // linkPreviewResultViewComponent, // slot: tool-result-renderers C5
  // messageBodyComponent, // slot: chat-element-library 8b
  // messageBubbleComponent, // slot: chat-element-library 8b
  // messageEditorComponent, // slot: message-branching 9
  // permissionRuleTileComponent, // slot: tool-approval-prompts 14b
  // planResultViewComponent, // slot: tool-result-renderers C5
  // slashCommandMenuComponent, // slot: chat-element-library 11a
  // sourcesResultViewComponent, // slot: tool-result-renderers C4
  // subagentBodyComponent, // slot: chat-element-library 7b
  // terminalResultViewComponent, // slot: tool-result-renderers C4
  // thinkingBodyComponent, // slot: chat-element-library 7a
  // threadViewportComponent, // slot: chat-element-library 10
  // timelineEntryShellComponent, // slot: chat-element-library 5b
  // timelineEntryViewComponent, // slot: chat-element-library 7c
  // typingIndicatorComponent, // slot: chat-element-library 8b
  // unknownInteractionCardComponent, // slot: tool-approval-prompts 13b
  approvalCardComponent,
  branchPickerComponent,
  codeBlockComponent,
  compactExpandableComponent,
  conversationListViewComponent,
  conversationSearchResultsViewComponent,
  feedbackButtonsComponent,
  feedbackCommentFieldComponent,
  genericToolResultViewComponent,
  highlightedSegmentsComponent,
  jsonTreeViewComponent,
  messageActionButtonComponent,
  messageActionsComponent,
  messageEditorComponent,
  messageTimingIndicatorComponent,
  permissionRuleTileComponent,
  questionCardComponent,
  queuedMessageBubbleComponent,
  regenerateMenuButtonComponent,
  starterPromptsComponent,
  suggestionChipsComponent,
  surfaceFrameComponent,
  threadDensityScopeComponent,
  threadEmptyStateComponent,
  threadErrorBannerComponent,
  timelineEntryShellComponent,
  toolCallBodyComponent,
  toolResultRendererScopeComponent,
  toolStatusIconComponent,
  turnProgressCardComponent,
  unknownInteractionCardComponent,
];
