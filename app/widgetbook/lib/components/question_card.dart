import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

import '../knobs.dart';

final _now = DateTime.utc(2026, 1, 15, 12);

const _questions = [
  QuestionItem(
    id: 'q1',
    prompt: 'Which branch should I deploy?',
    options: [
      QuestionOption(
        id: 'o1',
        label: 'main',
        description: 'The default branch',
      ),
      QuestionOption(id: 'o2', label: 'develop'),
    ],
  ),
  QuestionItem(
    id: 'q2',
    prompt: 'Which labels apply?',
    multiple: true,
    options: [
      QuestionOption(id: 'a', label: 'bug'),
      QuestionOption(id: 'b', label: 'ui'),
    ],
  ),
  QuestionItem(
    id: 'q3',
    prompt: 'What should the release be called?',
    freeText: true,
  ),
];

const _answers = [
  QuestionAnswer(questionId: 'q1', selectedOptionIds: ['o1']),
  QuestionAnswer(questionId: 'q2', selectedOptionIds: ['a', 'b']),
  QuestionAnswer(questionId: 'q3', text: 'Atlas'),
];

final questionCardComponent = WidgetbookComponent(
  name: 'QuestionCard',
  useCases: [
    WidgetbookUseCase(
      name: 'default',
      builder: (context) {
        final status = enumKnob(
          context,
          label: 'status',
          values: InteractionStatus.values,
          initial: InteractionStatus.pending,
        );
        final count = context.knobs.int.slider(
          label: 'questions',
          initialValue: 3,
          min: 1,
          max: 3,
        );
        final declined = context.knobs.boolean(
          label: 'declined (resolved)',
          initialValue: false,
        );
        return Padding(
          padding: const EdgeInsets.all(16),
          child: QuestionCard(
            now: _now,
            onAnswer: (_) {},
            onDecline: () {},
            record: InteractionRecord(
              id: 'i1',
              kind: InteractionKind.question,
              status: status,
              createdAt: _now,
              expiresAt: _now.add(const Duration(minutes: 9, seconds: 42)),
              questions: _questions.take(count).toList(),
              answers: status == InteractionStatus.resolved && !declined
                  ? _answers.take(count).toList()
                  : const [],
              declined: status == InteractionStatus.resolved && declined,
            ),
          ),
        );
      },
    ),
  ],
);
