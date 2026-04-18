import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for TaskStatusUpdateEvent
void main() {
  final instance = TaskStatusUpdateEventBuilder();
  // TODO add properties to the builder and call build()

  group(TaskStatusUpdateEvent, () {
    // The ID of the context that the task belongs to.
    // String contextId
    test('to test the property `contextId`', () async {
      // TODO
    });

    // Metadata associated with the task update.
    // BuiltMap<String, JsonObject> metadata
    test('to test the property `metadata`', () async {
      // TODO
    });

    // The new status of the task.
    // TaskStatus status
    test('to test the property `status`', () async {
      // TODO
    });

    // The ID of the task that has changed.
    // String taskId
    test('to test the property `taskId`', () async {
      // TODO
    });
  });
}
