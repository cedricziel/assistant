import 'package:assistant_ui/assistant_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

final WidgetbookComponent messageEditorComponent = WidgetbookComponent(
  name: 'MessageEditor',
  useCases: [
    WidgetbookUseCase(
      name: 'Default',
      builder: (context) => _frame(
        MessageEditor(
          initialText: context.knobs.string(
            label: 'initialText',
            initialValue: 'Summarise the quarterly report in three bullets.',
          ),
          onSubmit: (_) {},
          onCancel: () {},
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'With attachments',
      builder: (context) => _frame(
        MessageEditor(
          initialText: 'Summarise the attached files.',
          onSubmit: (_) {},
          onCancel: () {},
          attachments: const [
            Chip(label: Text('report.pdf')),
            Chip(label: Text('photo.png')),
          ],
        ),
      ),
    ),
  ],
);

Widget _frame(Widget child) => Center(
  child: Padding(
    padding: const EdgeInsets.all(16),
    child: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 480),
      child: child,
    ),
  ),
);
