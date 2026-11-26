import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

import 'package:assistant_widgetbook/components.dart';
import 'package:assistant_widgetbook/themes.dart';

void main() => runApp(const AssistantWidgetbook());

class AssistantWidgetbook extends StatelessWidget {
  const AssistantWidgetbook({super.key});

  @override
  Widget build(BuildContext context) {
    return Widgetbook(
      appBuilder: materialUiAppBuilder,
      addons: [kThemeAddon],
      directories: [WidgetbookFolder(name: 'Elements', children: kComponents)],
    );
  }
}
