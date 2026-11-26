import 'package:widgetbook/widgetbook.dart';

import 'components/message_action_button.dart';
import 'components/message_actions.dart';
import 'components/message_timing_indicator.dart';
import 'components/starter_prompts.dart';
import 'components/suggestion_chips.dart';

/// One component per `kAssistantUiElements` entry, named after the element and
/// sorted by name. Element PRs insert their own component in sorted position.
final List<WidgetbookComponent> kComponents = [
  messageActionButtonComponent,
  messageActionsComponent,
  messageTimingIndicatorComponent,
  starterPromptsComponent,
  suggestionChipsComponent,
];
