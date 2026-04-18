import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for ListTasksResponse
void main() {
  final instance = ListTasksResponseBuilder();
  // TODO add properties to the builder and call build()

  group(ListTasksResponse, () {
    // A token to retrieve the next page of results.
    // String nextPageToken
    test('to test the property `nextPageToken`', () async {
      // TODO
    });

    // The page size used for this response.
    // int pageSize
    test('to test the property `pageSize`', () async {
      // TODO
    });

    // Tasks matching the specified criteria.
    // BuiltList<Task> tasks
    test('to test the property `tasks`', () async {
      // TODO
    });

    // Total number of tasks available (before pagination).
    // int totalSize
    test('to test the property `totalSize`', () async {
      // TODO
    });
  });
}
