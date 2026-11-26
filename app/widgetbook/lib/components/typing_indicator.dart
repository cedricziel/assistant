import 'package:assistant_ui/assistant_ui.dart';
import 'package:widgetbook/widgetbook.dart';

final typingIndicatorComponent = WidgetbookComponent(
  name: 'TypingIndicator',
  useCases: [
    WidgetbookUseCase(
      name: 'default',
      builder: (context) => const TypingIndicator(),
    ),
  ],
);
