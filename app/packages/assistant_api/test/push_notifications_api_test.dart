import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for PushNotificationsApi
void main() {
  final instance = AssistantApi().getPushNotificationsApi();

  group(PushNotificationsApi, () {
    // `POST /tasks/:task_id/pushNotificationConfigs`
    //
    //Future<TaskPushNotificationConfig> createPushNotificationConfig(String taskId, CreateTaskPushNotificationConfigRequest createTaskPushNotificationConfigRequest) async
    test('test createPushNotificationConfig', () async {
      // TODO
    });

    // `DELETE /tasks/:task_id/pushNotificationConfigs/:config_id`
    //
    //Future deletePushNotificationConfig(String taskId, String configId) async
    test('test deletePushNotificationConfig', () async {
      // TODO
    });

    // `GET /tasks/:task_id/pushNotificationConfigs/:config_id`
    //
    //Future<TaskPushNotificationConfig> getPushNotificationConfig(String taskId, String configId) async
    test('test getPushNotificationConfig', () async {
      // TODO
    });

    // `GET /tasks/:task_id/pushNotificationConfigs`
    //
    //Future<ListTaskPushNotificationConfigsResponse> listPushNotificationConfigs(String taskId, { int pageSize, String pageToken }) async
    test('test listPushNotificationConfigs', () async {
      // TODO
    });
  });
}
