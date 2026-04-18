import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for TaskArtifactUpdateEvent
void main() {
  final instance = TaskArtifactUpdateEventBuilder();
  // TODO add properties to the builder and call build()

  group(TaskArtifactUpdateEvent, () {
    // If true, append content to a previously sent artifact with the same ID.
    // bool append
    test('to test the property `append`', () async {
      // TODO
    });

    // The artifact that was generated or updated.
    // Artifact artifact
    test('to test the property `artifact`', () async {
      // TODO
    });

    // The ID of the context that this task belongs to.
    // String contextId
    test('to test the property `contextId`', () async {
      // TODO
    });

    // If true, this is the final chunk of the artifact.
    // bool lastChunk
    test('to test the property `lastChunk`', () async {
      // TODO
    });

    // Metadata associated with the artifact update.
    // BuiltMap<String, JsonObject> metadata
    test('to test the property `metadata`', () async {
      // TODO
    });

    // The ID of the task for this artifact.
    // String taskId
    test('to test the property `taskId`', () async {
      // TODO
    });
  });
}
