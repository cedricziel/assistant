import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

final WidgetbookComponent regenerateMenuButtonComponent = WidgetbookComponent(
  name: 'RegenerateMenuButton',
  useCases: [
    WidgetbookUseCase(
      name: 'Default',
      builder: (context) {
        final count = context.knobs.int.slider(
          label: 'models',
          initialValue: 3,
          min: 0,
          max: 5,
        );
        return Align(
          alignment: Alignment.topLeft,
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: RegenerateMenuButton(
              models: [
                for (var i = 0; i < count; i++)
                  RegenerateModel(
                    name: 'model-${String.fromCharCode(97 + i)}',
                    isDefault: i == 0,
                  ),
              ],
              onSelected: (_) {},
            ),
          ),
        );
      },
    ),
  ],
);
