import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for ListTasksRequest
void main() {
  final instance = ListTasksRequestBuilder();
  // TODO add properties to the builder and call build()

  group(ListTasksRequest, () {
    // Filter tasks by context ID.
    // String contextId
    test('to test the property `contextId`', () async {
      // TODO
    });

    // Max number of messages to include in each task's history.
    // int historyLength
    test('to test the property `historyLength`', () async {
      // TODO
    });

    // Whether to include artifacts in returned tasks.
    // bool includeArtifacts
    test('to test the property `includeArtifacts`', () async {
      // TODO
    });

    // Max number of tasks to return (1..=100, default 50).
    // int pageSize
    test('to test the property `pageSize`', () async {
      // TODO
    });

    // Page token from a previous `ListTasks` call.
    // String pageToken
    test('to test the property `pageToken`', () async {
      // TODO
    });

    // Filter tasks by current status state.
    // TaskState status
    test('to test the property `status`', () async {
      // TODO
    });

    // Filter tasks with status updated after this timestamp.
    // DateTime statusTimestampAfter
    test('to test the property `statusTimestampAfter`', () async {
      // TODO
    });

    // Tenant ID.
    // String tenant
    test('to test the property `tenant`', () async {
      // TODO
    });
  });
}
