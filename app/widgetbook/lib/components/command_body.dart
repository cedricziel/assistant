import 'package:assistant_ui/assistant_ui.dart';
import 'package:widgetbook/widgetbook.dart';

final commandBodyComponent = WidgetbookComponent(
  name: 'CommandBody',
  useCases: [
    WidgetbookUseCase(
      name: 'playground',
      builder: (context) => CommandBody(
        name: context.knobs.string(label: 'name', initialValue: 'model'),
        ack: context.knobs.string(
          label: 'ack',
          initialValue: 'Model set to gpt-4o',
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'long acknowledgement',
      builder: (context) => CommandBody(
        name: 'compact',
        ack: 'History compacted: ${'older turns summarised ' * 6}',
      ),
    ),
  ],
);
