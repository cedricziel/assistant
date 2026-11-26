import 'package:widgetbook/widgetbook.dart';

import 'components/code_block.dart';
import 'components/starter_prompts.dart';
import 'components/suggestion_chips.dart';

/// One component per `kAssistantUiElements` entry, named after the element and
/// sorted by name. Element PRs append their own component here.
final List<WidgetbookComponent> kComponents = [
  codeBlockComponent,
  starterPromptsComponent,
  suggestionChipsComponent,
];
