import 'package:widgetbook/widgetbook.dart';

import 'components/conversation_list_view.dart';
import 'components/message_timing_indicator.dart';
import 'components/starter_prompts.dart';
import 'components/suggestion_chips.dart';

/// One component per `kAssistantUiElements` entry, named after the element and
/// sorted by name. Element PRs insert their own component in sorted position.
final List<WidgetbookComponent> kComponents = [
  conversationListViewComponent,
  messageTimingIndicatorComponent,
  starterPromptsComponent,
  suggestionChipsComponent,
];
