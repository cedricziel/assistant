import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

final _now = DateTime.utc(2026, 1, 15, 12);

const _branch = QuestionItem(
  id: 'q1',
  prompt: 'Which branch?',
  options: [
    QuestionOption(id: 'o1', label: 'main', description: 'The default'),
    QuestionOption(id: 'o2', label: 'develop'),
  ],
);

const _tags = QuestionItem(
  id: 'q2',
  prompt: 'Which tags?',
  multiple: true,
  options: [
    QuestionOption(id: 'a', label: 'bug'),
    QuestionOption(id: 'b', label: 'ui'),
    QuestionOption(id: 'c', label: 'docs'),
  ],
);

const _name = QuestionItem(
  id: 'q3',
  prompt: 'What is the name?',
  freeText: true,
);

InteractionRecord _record({
  List<QuestionItem> questions = const [_branch],
  InteractionStatus status = InteractionStatus.pending,
  List<QuestionAnswer> answers = const [],
  bool declined = false,
  DateTime? expiresAt,
}) => InteractionRecord(
  id: 'i1',
  kind: InteractionKind.question,
  status: status,
  createdAt: _now,
  expiresAt: expiresAt ?? _now.add(const Duration(minutes: 9, seconds: 42)),
  questions: questions,
  answers: answers,
  declined: declined,
);

Future<void> _pump(
  WidgetTester tester,
  InteractionRecord record, {
  ValueChanged<List<QuestionAnswer>>? onAnswer,
  VoidCallback? onDecline,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: SingleChildScrollView(
        child: QuestionCard(
          record: record,
          now: _now,
          onAnswer: onAnswer ?? (_) {},
          onDecline: onDecline ?? () {},
        ),
      ),
    ),
  ),
);

FilledButton _submit(WidgetTester tester) =>
    tester.widget<FilledButton>(find.byKey(const Key('question_submit')));

void main() {
  group('models', () {
    test('record defaults to no questions, answers or decline', () {
      final record = InteractionRecord(
        id: 'i',
        kind: InteractionKind.question,
        status: InteractionStatus.pending,
        createdAt: _now,
        expiresAt: _now,
      );
      expect(record.questions, isEmpty);
      expect(record.answers, isEmpty);
      expect(record.declined, isFalse);
    });

    test('QuestionItem defaults to single choice without free text', () {
      expect(_branch.multiple, isFalse);
      expect(_branch.freeText, isFalse);
    });
  });

  group('pending', () {
    testWidgets('labels the card as coming from the assistant', (tester) async {
      await _pump(tester, _record());

      expect(find.text('Your assistant has a question'), findsOneWidget);
      expect(find.text('Which branch?'), findsOneWidget);
      expect(find.text('Expires in 9:42'), findsOneWidget);
    });

    testWidgets('single choice renders radios with descriptions', (
      tester,
    ) async {
      await _pump(tester, _record());

      expect(find.byType(RadioListTile<String>), findsNWidgets(2));
      expect(find.byType(CheckboxListTile), findsNothing);
      expect(find.text('main'), findsOneWidget);
      expect(find.text('The default'), findsOneWidget);
      expect(find.text('develop'), findsOneWidget);
    });

    testWidgets('multiple choice renders checkboxes', (tester) async {
      await _pump(tester, _record(questions: const [_tags]));

      expect(find.byType(CheckboxListTile), findsNWidgets(3));
      expect(find.byType(RadioListTile<String>), findsNothing);
    });

    testWidgets('free text renders a text field', (tester) async {
      await _pump(tester, _record(questions: const [_name]));

      expect(find.byType(TextField), findsOneWidget);
      expect(find.byType(RadioListTile<String>), findsNothing);
    });

    testWidgets('options plus free text render both', (tester) async {
      const question = QuestionItem(
        id: 'q',
        prompt: 'Where?',
        freeText: true,
        options: [QuestionOption(id: 'o', label: 'here')],
      );
      await _pump(tester, _record(questions: const [question]));

      expect(find.byType(RadioListTile<String>), findsOneWidget);
      expect(find.byType(TextField), findsOneWidget);
    });

    testWidgets('submit is disabled until the question is answered', (
      tester,
    ) async {
      await _pump(tester, _record());
      expect(_submit(tester).onPressed, isNull);

      await tester.tap(find.text('main'));
      await tester.pump();

      expect(_submit(tester).onPressed, isNotNull);
    });

    testWidgets('submit stays disabled until every question is answered', (
      tester,
    ) async {
      await _pump(tester, _record(questions: const [_branch, _tags, _name]));

      await tester.tap(find.text('main'));
      await tester.pump();
      expect(_submit(tester).onPressed, isNull);

      await tester.tap(find.text('bug'));
      await tester.pump();
      expect(_submit(tester).onPressed, isNull);

      await tester.enterText(find.byType(TextField), 'Atlas');
      await tester.pump();
      expect(_submit(tester).onPressed, isNotNull);
    });

    testWidgets('whitespace-only text does not count as an answer', (
      tester,
    ) async {
      await _pump(tester, _record(questions: const [_name]));

      await tester.enterText(find.byType(TextField), '   ');
      await tester.pump();

      expect(_submit(tester).onPressed, isNull);
    });

    testWidgets('unchecking the last checkbox disables submit again', (
      tester,
    ) async {
      await _pump(tester, _record(questions: const [_tags]));

      await tester.tap(find.text('bug'));
      await tester.pump();
      expect(_submit(tester).onPressed, isNotNull);

      await tester.tap(find.text('bug'));
      await tester.pump();
      expect(_submit(tester).onPressed, isNull);
    });

    testWidgets('onAnswer receives structured answers in question order', (
      tester,
    ) async {
      List<QuestionAnswer>? received;
      await _pump(
        tester,
        _record(questions: const [_branch, _tags, _name]),
        onAnswer: (answers) => received = answers,
      );

      await tester.tap(find.text('develop'));
      await tester.tap(find.text('bug'));
      await tester.tap(find.text('docs'));
      await tester.enterText(find.byType(TextField), ' Atlas ');
      await tester.pump();
      await tester.tap(find.byKey(const Key('question_submit')));

      expect(received, isNotNull);
      expect(received!.map((a) => a.questionId), ['q1', 'q2', 'q3']);
      expect(received![0].selectedOptionIds, ['o2']);
      expect(received![0].text, isNull);
      expect(received![1].selectedOptionIds, ['a', 'c']);
      expect(received![2].selectedOptionIds, isEmpty);
      expect(received![2].text, 'Atlas');
    });

    testWidgets('radio selection can change before submitting', (tester) async {
      List<QuestionAnswer>? received;
      await _pump(tester, _record(), onAnswer: (a) => received = a);

      await tester.tap(find.text('main'));
      await tester.tap(find.text('develop'));
      await tester.pump();
      await tester.tap(find.byKey(const Key('question_submit')));

      expect(received!.single.selectedOptionIds, ['o2']);
    });

    testWidgets('onDecline fires without any answer', (tester) async {
      var declined = 0;
      await _pump(tester, _record(), onDecline: () => declined++);

      await tester.tap(find.byKey(const Key('question_decline')));

      expect(declined, 1);
    });

    testWidgets('free text is capped at 4000 characters', (tester) async {
      await _pump(tester, _record(questions: const [_name]));

      final field = tester.widget<TextField>(find.byType(TextField));
      expect(field.maxLength, 4000);
    });

    testWidgets('expired countdown reads Expiring…', (tester) async {
      await _pump(tester, _record(expiresAt: _now));

      expect(find.text('Expiring…'), findsOneWidget);
    });
  });

  group('submitting', () {
    testWidgets('disables inputs and both actions', (tester) async {
      await _pump(
        tester,
        _record(
          questions: const [_branch, _name],
          status: InteractionStatus.submitting,
        ),
      );

      expect(_submit(tester).onPressed, isNull);
      expect(
        tester
            .widget<TextButton>(find.byKey(const Key('question_decline')))
            .onPressed,
        isNull,
      );
      expect(tester.widget<TextField>(find.byType(TextField)).enabled, isFalse);
    });
  });

  group('receipts', () {
    testWidgets('resolved shows the chosen answers and no inputs', (
      tester,
    ) async {
      await _pump(
        tester,
        _record(
          questions: const [_branch, _tags, _name],
          status: InteractionStatus.resolved,
          answers: const [
            QuestionAnswer(questionId: 'q1', selectedOptionIds: ['o1']),
            QuestionAnswer(questionId: 'q2', selectedOptionIds: ['a', 'b']),
            QuestionAnswer(questionId: 'q3', text: 'Atlas'),
          ],
        ),
      );

      expect(find.text('Answered'), findsOneWidget);
      expect(find.text('Which branch?'), findsOneWidget);
      expect(find.text('main'), findsOneWidget);
      expect(find.text('bug, ui'), findsOneWidget);
      expect(find.text('Atlas'), findsOneWidget);
      expect(find.byType(RadioListTile<String>), findsNothing);
      expect(find.byType(CheckboxListTile), findsNothing);
      expect(find.byType(TextField), findsNothing);
      expect(find.byKey(const Key('question_submit')), findsNothing);
      expect(find.byKey(const Key('question_decline')), findsNothing);
    });

    testWidgets('resolved with an unknown option id shows no label for it', (
      tester,
    ) async {
      await _pump(
        tester,
        _record(
          status: InteractionStatus.resolved,
          answers: const [
            QuestionAnswer(questionId: 'q1', selectedOptionIds: ['gone']),
          ],
        ),
      );

      expect(find.text('Answered'), findsOneWidget);
      expect(find.text('gone'), findsNothing);
    });

    for (final (title, record) in [
      ('Declined', _record(status: InteractionStatus.resolved, declined: true)),
      ('Expired', _record(status: InteractionStatus.expired)),
      ('Cancelled', _record(status: InteractionStatus.cancelled)),
    ]) {
      testWidgets('$title is a read-only receipt', (tester) async {
        await _pump(tester, record);

        expect(find.text(title), findsOneWidget);
        expect(find.text('Which branch?'), findsOneWidget);
        expect(find.byType(RadioListTile<String>), findsNothing);
        expect(find.byKey(const Key('question_submit')), findsNothing);
        expect(find.byKey(const Key('question_decline')), findsNothing);
      });
    }
  });
}
