# assistant_api.api.PushNotificationsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createPushNotificationConfig**](PushNotificationsApi.md#createpushnotificationconfig) | **POST** /tasks/{task_id}/pushNotificationConfigs | &#x60;POST /tasks/:task_id/pushNotificationConfigs&#x60;
[**deletePushNotificationConfig**](PushNotificationsApi.md#deletepushnotificationconfig) | **DELETE** /tasks/{task_id}/pushNotificationConfigs/{config_id} | &#x60;DELETE /tasks/:task_id/pushNotificationConfigs/:config_id&#x60;
[**getPushNotificationConfig**](PushNotificationsApi.md#getpushnotificationconfig) | **GET** /tasks/{task_id}/pushNotificationConfigs/{config_id} | &#x60;GET /tasks/:task_id/pushNotificationConfigs/:config_id&#x60;
[**listPushNotificationConfigs**](PushNotificationsApi.md#listpushnotificationconfigs) | **GET** /tasks/{task_id}/pushNotificationConfigs | &#x60;GET /tasks/:task_id/pushNotificationConfigs&#x60;


# **createPushNotificationConfig**
> TaskPushNotificationConfig createPushNotificationConfig(taskId, createTaskPushNotificationConfigRequest)

`POST /tasks/:task_id/pushNotificationConfigs`

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPushNotificationsApi();
final String taskId = taskId_example; // String | Parent task ID
final CreateTaskPushNotificationConfigRequest createTaskPushNotificationConfigRequest = ; // CreateTaskPushNotificationConfigRequest | 

try {
    final response = api.createPushNotificationConfig(taskId, createTaskPushNotificationConfigRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PushNotificationsApi->createPushNotificationConfig: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **taskId** | **String**| Parent task ID | 
 **createTaskPushNotificationConfigRequest** | [**CreateTaskPushNotificationConfigRequest**](CreateTaskPushNotificationConfigRequest.md)|  | 

### Return type

[**TaskPushNotificationConfig**](TaskPushNotificationConfig.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deletePushNotificationConfig**
> deletePushNotificationConfig(taskId, configId)

`DELETE /tasks/:task_id/pushNotificationConfigs/:config_id`

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPushNotificationsApi();
final String taskId = taskId_example; // String | Parent task ID
final String configId = configId_example; // String | Push notification config ID

try {
    api.deletePushNotificationConfig(taskId, configId);
} on DioException catch (e) {
    print('Exception when calling PushNotificationsApi->deletePushNotificationConfig: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **taskId** | **String**| Parent task ID | 
 **configId** | **String**| Push notification config ID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getPushNotificationConfig**
> TaskPushNotificationConfig getPushNotificationConfig(taskId, configId)

`GET /tasks/:task_id/pushNotificationConfigs/:config_id`

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPushNotificationsApi();
final String taskId = taskId_example; // String | Parent task ID
final String configId = configId_example; // String | Push notification config ID

try {
    final response = api.getPushNotificationConfig(taskId, configId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PushNotificationsApi->getPushNotificationConfig: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **taskId** | **String**| Parent task ID | 
 **configId** | **String**| Push notification config ID | 

### Return type

[**TaskPushNotificationConfig**](TaskPushNotificationConfig.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listPushNotificationConfigs**
> ListTaskPushNotificationConfigsResponse listPushNotificationConfigs(taskId, pageSize, pageToken)

`GET /tasks/:task_id/pushNotificationConfigs`

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPushNotificationsApi();
final String taskId = taskId_example; // String | Parent task ID
final int pageSize = 56; // int | 
final String pageToken = pageToken_example; // String | 

try {
    final response = api.listPushNotificationConfigs(taskId, pageSize, pageToken);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PushNotificationsApi->listPushNotificationConfigs: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **taskId** | **String**| Parent task ID | 
 **pageSize** | **int**|  | [optional] 
 **pageToken** | **String**|  | [optional] 

### Return type

[**ListTaskPushNotificationConfigsResponse**](ListTaskPushNotificationConfigsResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

