/// Pure, state-free chat elements.
///
/// See `README.md` for the purity, injection and composition rules.
library;

// export 'src/bodies/command_body.dart'; // slot: chat-element-library 7b

// export 'src/bodies/subagent_body.dart'; // slot: chat-element-library 7b
// export 'src/branch_picker.dart'; // slot: message-branching 9
// export 'src/code/code_block.dart'; // slot: chat-a11y-and-code-blocks C4
// export 'src/composer/attachment_tray.dart'; // slot: chat-element-library 11b
// export 'src/composer/composer.dart'; // slot: chat-element-library 11a
// export 'src/composer/composer_voice_button.dart'; // slot: chat-element-library 11b
export 'src/bodies/thinking_body.dart';
export 'src/bodies/tool_call_body.dart';
export 'src/bodies/tool_status_icon.dart';
export 'src/branch_picker.dart';
export 'src/code/code_block.dart';
export 'src/composer/queued_message_bubble.dart';
export 'src/conversations/conversation_list_view.dart';
export 'src/feedback/feedback_buttons.dart';
export 'src/feedback/feedback_comment_field.dart';
export 'src/highlight/code_token.dart';
export 'src/highlight/highlight_code.dart';
export 'src/interactions/approval_card.dart';
export 'src/interactions/permission_rule_tile.dart';
export 'src/interactions/question_card.dart';
export 'src/interactions/unknown_interaction_card.dart';
export 'src/message_actions.dart';
export 'src/message_editor.dart';
export 'src/registry.dart';
export 'src/scopes/chat_markdown_scope.dart';
export 'src/scopes/thread_density_scope.dart';
export 'src/scopes/tool_result_renderer_scope.dart';
export 'src/suggestions/starter_prompts.dart';
export 'src/suggestions/suggestion_chips.dart';
export 'src/theme/assistant_ui_colors.dart';
export 'src/theme/assistant_ui_spacing.dart';
export 'src/theme/code_theme.dart';
export 'src/timeline/timeline_entry_shell.dart';
export 'src/timing/message_timing_indicator.dart';
export 'src/tool_results/generic_tool_result_view.dart';

// export 'src/tool_results/code_diff_result_view.dart'; // slot: tool-result-renderers C7

// export 'src/tool_results/code_file_result_view.dart'; // slot: tool-result-renderers C7

// export 'src/tool_results/compact_expandable.dart'; // slot: tool-result-renderers C2

// export 'src/tool_results/data_table_result_view.dart'; // slot: tool-result-renderers C6

// export 'src/tool_results/json_tree_view.dart'; // slot: tool-result-renderers C2b

// export 'src/tool_results/link_preview_result_view.dart'; // slot: tool-result-renderers C5

// export 'src/tool_results/plan_result_view.dart'; // slot: tool-result-renderers C5

// export 'src/tool_results/sources_result_view.dart'; // slot: tool-result-renderers C4

// export 'src/tool_results/terminal_result_view.dart'; // slot: tool-result-renderers C4

// export 'src/turn/turn_progress_card.dart'; // slot: chat-element-library 9
