import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

final unknownInteractionCardComponent = WidgetbookComponent(
  name: 'UnknownInteractionCard',
  useCases: [
    WidgetbookUseCase(
      name: 'default',
      builder: (context) => const Padding(
        padding: EdgeInsets.all(16),
        child: UnknownInteractionCard(),
      ),
    ),
  ],
);
