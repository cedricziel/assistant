import 'package:assistant_ui/assistant_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

import 'package:assistant_widgetbook/knobs.dart';

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

final messageActionsComponent = WidgetbookComponent(
  name: 'MessageActions',
  useCases: [
    WidgetbookUseCase(
      name: 'Full row',
      builder: (context) => Padding(
        padding: const EdgeInsets.all(16),
        child: MessageActions(
          leading: const Text('1 / 2'),
          actions: [
            MessageActionButton(
              icon: Icons.copy_outlined,
              label: 'Copy',
              onPressed: () {},
            ),
            MessageActionButton(
              icon: Icons.thumb_up,
              label: 'Good',
              selected: context.knobs.boolean(label: 'Rated good'),
              onPressed: () {},
            ),
            MessageActionButton(
              icon: Icons.refresh,
              label: 'Retry',
              tone: MessageActionTone.danger,
              onPressed: () {},
            ),
          ],
          trailing: const [Text('1.2 s')],
        ),
      ),
    ),
  ],
);
