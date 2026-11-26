@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

const _branch = QuestionItem(
  id: 'q1',
  prompt: 'Which branch should I deploy?',
  options: [
    QuestionOption(id: 'o1', label: 'main', description: 'The default branch'),
    QuestionOption(id: 'o2', label: 'develop'),
  ],
);

const _tags = QuestionItem(
  id: 'q2',
  prompt: 'Which labels apply?',
  multiple: true,
  options: [
    QuestionOption(id: 'a', label: 'bug'),
    QuestionOption(id: 'b', label: 'ui'),
    QuestionOption(id: 'c', label: 'docs'),
  ],
);

const _name = QuestionItem(
  id: 'q3',
  prompt: 'What should the release be called?',
  freeText: true,
);

InteractionRecord _record(
  List<QuestionItem> questions, {
  InteractionStatus status = InteractionStatus.pending,
  List<QuestionAnswer> answers = const [],
  bool declined = false,
}) => InteractionRecord(
  id: 'i1',
  kind: InteractionKind.question,
  status: status,
  createdAt: kGoldenNow,
  expiresAt: kGoldenNow.add(const Duration(minutes: 9, seconds: 42)),
  questions: questions,
  answers: answers,
  declined: declined,
);

void main() {
  final cases = <String, ({InteractionRecord record, Size size})>{
    'question_card_single': (
      record: _record(const [_branch]),
      size: const Size(400, 420),
    ),
    'question_card_multiple': (
      record: _record(const [_tags]),
      size: const Size(400, 440),
    ),
    'question_card_free_text': (
      record: _record(const [_name]),
      size: const Size(400, 360),
    ),
    'question_card_multi_question': (
      record: _record(const [_branch, _tags, _name]),
      size: const Size(400, 900),
    ),
    'question_card_submitting': (
      record: _record(
        const [_branch],
        status: InteractionStatus.submitting,
        answers: const [
          QuestionAnswer(questionId: 'q1', selectedOptionIds: ['o1']),
        ],
      ),
      size: const Size(400, 420),
    ),
    'question_card_answered': (
      record: _record(
        const [_branch, _tags, _name],
        status: InteractionStatus.resolved,
        answers: const [
          QuestionAnswer(questionId: 'q1', selectedOptionIds: ['o1']),
          QuestionAnswer(questionId: 'q2', selectedOptionIds: ['a', 'b']),
          QuestionAnswer(questionId: 'q3', text: 'Atlas'),
        ],
      ),
      size: const Size(400, 300),
    ),
    'question_card_declined': (
      record: _record(
        const [_branch],
        status: InteractionStatus.resolved,
        declined: true,
      ),
      size: const Size(400, 140),
    ),
    'question_card_expired': (
      record: _record(const [_branch], status: InteractionStatus.expired),
      size: const Size(400, 140),
    ),
    'question_card_cancelled': (
      record: _record(const [_branch], status: InteractionStatus.cancelled),
      size: const Size(400, 140),
    ),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          Padding(
            padding: const EdgeInsets.all(16),
            child: QuestionCard(
              record: entry.value.record,
              now: kGoldenNow,
              onAnswer: (_) {},
              onDecline: () {},
            ),
          ),
          size: entry.value.size,
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 50));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
