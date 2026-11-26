import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

WidgetbookUseCase _useCase(
  String name, {
  MessageRating? rating,
  bool enabled = true,
}) => WidgetbookUseCase(
  name: name,
  builder: (context) => Padding(
    padding: const EdgeInsets.all(16),
    child: FeedbackButtons(rating: rating, onChanged: (_) {}, enabled: enabled),
  ),
);

final feedbackButtonsComponent = WidgetbookComponent(
  name: 'FeedbackButtons',
  useCases: [
    _useCase('none'),
    _useCase('positive', rating: MessageRating.positive),
    _useCase('negative', rating: MessageRating.negative),
    _useCase('disabled', rating: MessageRating.positive, enabled: false),
  ],
);
