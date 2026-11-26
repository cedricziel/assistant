import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for QuestionAnswer
void main() {
  final instance = QuestionAnswerBuilder();
  // TODO add properties to the builder and call build()

  group(QuestionAnswer, () {
    // Id of the question being answered.
    // String questionId
    test('to test the property `questionId`', () async {
      // TODO
    });

    // Ids of the chosen options. Empty when the answer is free text only.
    // BuiltList<String> selectedOptionIds
    test('to test the property `selectedOptionIds`', () async {
      // TODO
    });

    // Free-text answer, at most 4000 characters, where the question allows it.
    // String text
    test('to test the property `text`', () async {
      // TODO
    });
  });
}
