import 'package:widgetbook/widgetbook.dart';

import 'package:assistant_widgetbook/components/branch_picker.dart';
import 'package:assistant_widgetbook/components/message_editor.dart';

/// One component per `kAssistantUiElements` entry, named after the element and
/// sorted by name. Element PRs append their own component here.
final List<WidgetbookComponent> kComponents = [
  branchPickerComponent,
  messageEditorComponent,
];
