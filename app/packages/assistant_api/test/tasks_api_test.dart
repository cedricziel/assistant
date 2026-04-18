import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for TasksApi
void main() {
  final instance = AssistantApi().getTasksApi();

  group(TasksApi, () {
    // `POST /tasks/:id/cancel` -- Cancels a task.
    //
    //Future<Task> cancelTask(String id) async
    test('test cancelTask', () async {
      // TODO
    });

    // `GET /tasks/:id` -- Gets the latest state of a task.
    //
    //Future<Task> getTask(String id) async
    test('test getTask', () async {
      // TODO
    });

    // `GET /tasks` -- Lists tasks matching optional filters.
    //
    //Future<ListTasksResponse> listTasks({ String contextId, TaskState status, int pageSize }) async
    test('test listTasks', () async {
      // TODO
    });

    // `GET /tasks/:id/subscribe` -- Subscribes to task updates (SSE).
    //
    //Future subscribeToTask(String id) async
    test('test subscribeToTask', () async {
      // TODO
    });
  });
}
