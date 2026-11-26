import 'package:widgetbook/widgetbook.dart';

import 'components/approval_card.dart';
import 'components/conversation_search_results_view.dart';
import 'components/highlighted_segments.dart';
import 'components/message_action_button.dart';
import 'components/message_actions.dart';
import 'components/message_timing_indicator.dart';
import 'components/starter_prompts.dart';
import 'components/suggestion_chips.dart';
import 'components/thread_density_scope.dart';

/// One component per `kAssistantUiElements` entry, named after the element and
/// sorted by name. Element PRs insert their own component in sorted position.
final List<WidgetbookComponent> kComponents = [
  approvalCardComponent,
  conversationSearchResultsViewComponent,
  highlightedSegmentsComponent,
  messageActionButtonComponent,
  messageActionsComponent,
  messageTimingIndicatorComponent,
  starterPromptsComponent,
  suggestionChipsComponent,
  threadDensityScopeComponent,
];
