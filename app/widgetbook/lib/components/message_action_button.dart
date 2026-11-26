import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_widgetbook/knobs.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

final messageActionButtonComponent = WidgetbookComponent(
  name: 'MessageActionButton',
  useCases: [
    WidgetbookUseCase(
      name: 'Playground',
      builder: (context) => Padding(
        padding: const EdgeInsets.all(16),
        child: MessageActionButton(
          icon: Icons.thumb_up_outlined,
          label: context.knobs.string(label: 'Label', initialValue: 'Good'),
          tone: enumKnob(
            context,
            label: 'Tone',
            values: MessageActionTone.values,
          ),
          selected: context.knobs.boolean(label: 'Selected'),
          onPressed: () {},
        ),
      ),
    ),
  ],
);
