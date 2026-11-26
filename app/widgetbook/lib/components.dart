import 'package:widgetbook/widgetbook.dart';

import 'components/approval_card.dart';
import 'components/branch_picker.dart';
import 'components/conversation_list_view.dart';
import 'components/feedback_buttons.dart';
import 'components/feedback_comment_field.dart';
import 'components/message_action_button.dart';
import 'components/message_actions.dart';
import 'components/message_editor.dart';
import 'components/message_timing_indicator.dart';
import 'components/permission_rule_tile.dart';
import 'components/question_card.dart';
import 'components/queued_message_bubble.dart';
import 'components/starter_prompts.dart';
import 'components/suggestion_chips.dart';
import 'components/surface_frame.dart';
import 'components/thread_density_scope.dart';
import 'components/timeline_entry_shell.dart';
import 'components/unknown_interaction_card.dart';

// import 'components/thread_empty_state.dart'; // slot: chat-element-library 9

// import 'components/thread_error_banner.dart'; // slot: chat-element-library 9

// import 'components/thread_viewport.dart'; // slot: chat-element-library 10

// import 'components/timeline_entry_shell.dart'; // slot: chat-element-library 5b

// import 'components/timeline_entry_view.dart'; // slot: chat-element-library 7c

// import 'components/tool_call_body.dart'; // slot: chat-element-library 6

// import 'components/tool_status_icon.dart'; // slot: chat-element-library 6

// import 'components/turn_progress_card.dart'; // slot: chat-element-library 9

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
  // codeBlockComponent, // slot: chat-a11y-and-code-blocks C4
  // codeDiffResultViewComponent, // slot: tool-result-renderers C7
  // codeFileResultViewComponent, // slot: tool-result-renderers C7
  // commandBodyComponent, // slot: chat-element-library 7b
  // compactExpandableComponent, // slot: tool-result-renderers C2
  // composerComponent, // slot: chat-element-library 11a
  // composerVoiceButtonComponent, // slot: chat-element-library 11b
  // conversationSearchResultsViewComponent, // slot: conversation-list-management 14
  // dataTableResultViewComponent, // slot: tool-result-renderers C6
  // feedbackButtonsComponent, // slot: message-feedback 5
  // feedbackCommentFieldComponent, // slot: message-feedback 5
  // genericToolResultViewComponent, // slot: chat-element-library 6
  // highlightedSegmentsComponent, // slot: conversation-list-management 14
  // jsonTreeViewComponent, // slot: tool-result-renderers C2b
  // linkPreviewResultViewComponent, // slot: tool-result-renderers C5
  // messageBodyComponent, // slot: chat-element-library 8b
  // messageBubbleComponent, // slot: chat-element-library 8b
  // messageEditorComponent, // slot: message-branching 9
  // permissionRuleTileComponent, // slot: tool-approval-prompts 14b
  // planResultViewComponent, // slot: tool-result-renderers C5
  // regenerateMenuButtonComponent, // slot: message-branching 10
  // slashCommandMenuComponent, // slot: chat-element-library 11a
  // sourcesResultViewComponent, // slot: tool-result-renderers C4
  // subagentBodyComponent, // slot: chat-element-library 7b
  // terminalResultViewComponent, // slot: tool-result-renderers C4
  // thinkingBodyComponent, // slot: chat-element-library 7a
  // threadEmptyStateComponent, // slot: chat-element-library 9
  // threadErrorBannerComponent, // slot: chat-element-library 9
  // threadViewportComponent, // slot: chat-element-library 10
  // timelineEntryShellComponent, // slot: chat-element-library 5b
  // timelineEntryViewComponent, // slot: chat-element-library 7c
  // toolCallBodyComponent, // slot: chat-element-library 6
  // toolStatusIconComponent, // slot: chat-element-library 6
  // turnProgressCardComponent, // slot: chat-element-library 9
  // typingIndicatorComponent, // slot: chat-element-library 8b
  // unknownInteractionCardComponent, // slot: tool-approval-prompts 13b
  approvalCardComponent,
  branchPickerComponent,
  conversationListViewComponent,
  feedbackButtonsComponent,
  feedbackCommentFieldComponent,
  messageActionButtonComponent,
  messageActionsComponent,
  messageEditorComponent,
  messageTimingIndicatorComponent,
  permissionRuleTileComponent,
  questionCardComponent,
  queuedMessageBubbleComponent,
  starterPromptsComponent,
  suggestionChipsComponent,
  surfaceFrameComponent,
  threadDensityScopeComponent,
  timelineEntryShellComponent,
  unknownInteractionCardComponent,
];
