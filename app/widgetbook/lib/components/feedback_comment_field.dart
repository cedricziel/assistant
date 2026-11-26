import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

WidgetbookUseCase _useCase(
  String name, {
  String? initialText,
  bool sending = false,
}) => WidgetbookUseCase(
  name: name,
  builder: (context) => Padding(
    padding: const EdgeInsets.all(16),
    child: FeedbackCommentField(
      initialText: initialText,
      sending: sending,
      onSubmit: (_) {},
      onDismiss: () {},
    ),
  ),
);

final feedbackCommentFieldComponent = WidgetbookComponent(
  name: 'FeedbackCommentField',
  useCases: [
    _useCase('empty'),
    _useCase('prefilled', initialText: 'Wrong file, it should be the config.'),
    _useCase('sending', initialText: 'Clear and short.', sending: true),
  ],
);
