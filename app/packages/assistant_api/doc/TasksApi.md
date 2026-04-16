# assistant_api.api.TasksApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cancelTask**](TasksApi.md#canceltask) | **POST** /tasks/{id}/cancel | &#x60;POST /tasks/:id/cancel&#x60; -- Cancels a task.
[**getTask**](TasksApi.md#gettask) | **GET** /tasks/{id} | &#x60;GET /tasks/:id&#x60; -- Gets the latest state of a task.
[**listTasks**](TasksApi.md#listtasks) | **GET** /tasks | &#x60;GET /tasks&#x60; -- Lists tasks matching optional filters.
[**subscribeToTask**](TasksApi.md#subscribetotask) | **GET** /tasks/{id}/subscribe | &#x60;GET /tasks/:id/subscribe&#x60; -- Subscribes to task updates (SSE).


# **cancelTask**
> Task cancelTask(id, cancelTaskRequest)

`POST /tasks/:id/cancel` -- Cancels a task.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getTasksApi();
final String id = id_example; // String | Task ID to cancel
final CancelTaskRequest cancelTaskRequest = ; // CancelTaskRequest | Optional cancellation metadata

try {
    final response = api.cancelTask(id, cancelTaskRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling TasksApi->cancelTask: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Task ID to cancel | 
 **cancelTaskRequest** | [**CancelTaskRequest**](CancelTaskRequest.md)| Optional cancellation metadata | 

### Return type

[**Task**](Task.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getTask**
> Task getTask(id, historyLength, tenant)

`GET /tasks/:id` -- Gets the latest state of a task.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getTasksApi();
final String id = id_example; // String | Task ID
final int historyLength = 56; // int | Max number of recent messages to include in history.
final String tenant = tenant_example; // String | Optional tenant ID.

try {
    final response = api.getTask(id, historyLength, tenant);
    print(response);
} on DioException catch (e) {
    print('Exception when calling TasksApi->getTask: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Task ID | 
 **historyLength** | **int**| Max number of recent messages to include in history. | [optional] 
 **tenant** | **String**| Optional tenant ID. | [optional] 

### Return type

[**Task**](Task.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listTasks**
> ListTasksResponse listTasks(tenant, contextId, status, pageSize, pageToken, historyLength, statusTimestampAfter, includeArtifacts)

`GET /tasks` -- Lists tasks matching optional filters.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getTasksApi();
final String tenant = tenant_example; // String | Tenant ID.
final String contextId = contextId_example; // String | Filter tasks by context ID.
final TaskState status = ; // TaskState | Filter tasks by current status state.
final int pageSize = 56; // int | Max number of tasks to return (1..=100, default 50).
final String pageToken = pageToken_example; // String | Page token from a previous `ListTasks` call.
final int historyLength = 56; // int | Max number of messages to include in each task's history.
final DateTime statusTimestampAfter = 2013-10-20T19:20:30+01:00; // DateTime | Filter tasks with status updated after this timestamp.
final bool includeArtifacts = true; // bool | Whether to include artifacts in returned tasks.

try {
    final response = api.listTasks(tenant, contextId, status, pageSize, pageToken, historyLength, statusTimestampAfter, includeArtifacts);
    print(response);
} on DioException catch (e) {
    print('Exception when calling TasksApi->listTasks: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **tenant** | **String**| Tenant ID. | [optional] 
 **contextId** | **String**| Filter tasks by context ID. | [optional] 
 **status** | [**TaskState**](.md)| Filter tasks by current status state. | [optional] 
 **pageSize** | **int**| Max number of tasks to return (1..=100, default 50). | [optional] 
 **pageToken** | **String**| Page token from a previous `ListTasks` call. | [optional] 
 **historyLength** | **int**| Max number of messages to include in each task's history. | [optional] 
 **statusTimestampAfter** | **DateTime**| Filter tasks with status updated after this timestamp. | [optional] 
 **includeArtifacts** | **bool**| Whether to include artifacts in returned tasks. | [optional] 

### Return type

[**ListTasksResponse**](ListTasksResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **subscribeToTask**
> StreamResponse subscribeToTask(id)

`GET /tasks/:id/subscribe` -- Subscribes to task updates (SSE).

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getTasksApi();
final String id = id_example; // String | Task ID to subscribe to

try {
    final response = api.subscribeToTask(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling TasksApi->subscribeToTask: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Task ID to subscribe to | 

### Return type

[**StreamResponse**](StreamResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: text/event-stream, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

