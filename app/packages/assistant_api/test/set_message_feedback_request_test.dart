import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for SetMessageFeedbackRequest
void main() {
  final instance = SetMessageFeedbackRequestBuilder();
  // TODO add properties to the builder and call build()

  group(SetMessageFeedbackRequest, () {
    // Optional free-text reason. Trimmed; an empty result means no comment. At most 2000 Unicode scalar values. Omitting it clears a stored comment.
    // String comment
    test('to test the property `comment`', () async {
      // TODO
    });

    // Thumbs up or down.
    // FeedbackRating rating
    test('to test the property `rating`', () async {
      // TODO
    });
  });
}
