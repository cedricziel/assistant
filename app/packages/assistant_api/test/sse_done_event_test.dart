import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for SseDoneEvent
void main() {
  final instance = SseDoneEventBuilder();
  // TODO add properties to the builder and call build()

  group(SseDoneEvent, () {
    // Attachments the reply produced. Omitted when there are none.
    // BuiltList<String> attachmentIds
    test('to test the property `attachmentIds`', () async {
      // TODO
    });

    // The full reply text.
    // String content
    test('to test the property `content`', () async {
      // TODO
    });

    // The saved assistant message. Absent when the turn saved none.
    // String messageId
    test('to test the property `messageId`', () async {
      // TODO
    });

    // Always `assistant`.
    // String role
    test('to test the property `role`', () async {
      // TODO
    });

    // How long the turn took. Absent when none was measured.
    // MessageTimingSummary timing
    test('to test the property `timing`', () async {
      // TODO
    });
  });
}
