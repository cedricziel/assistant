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
> Task cancelTask(id)

`POST /tasks/:id/cancel` -- Cancels a task.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getTasksApi();
final String id = id_example; // String | Task ID to cancel

try {
    final response = api.cancelTask(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling TasksApi->cancelTask: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Task ID to cancel | 

### Return type

[**Task**](Task.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getTask**
> Task getTask(id)

`GET /tasks/:id` -- Gets the latest state of a task.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getTasksApi();
final String id = id_example; // String | Task ID

try {
    final response = api.getTask(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling TasksApi->getTask: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Task ID | 

### Return type

[**Task**](Task.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listTasks**
> ListTasksResponse listTasks(contextId, status, pageSize)

`GET /tasks` -- Lists tasks matching optional filters.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getTasksApi();
final String contextId = contextId_example; // String | 
final TaskState status = ; // TaskState | 
final int pageSize = 56; // int | 

try {
    final response = api.listTasks(contextId, status, pageSize);
    print(response);
} on DioException catch (e) {
    print('Exception when calling TasksApi->listTasks: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **contextId** | **String**|  | [optional] 
 **status** | [**TaskState**](.md)|  | [optional] 
 **pageSize** | **int**|  | [optional] 

### Return type

[**ListTasksResponse**](ListTasksResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **subscribeToTask**
> subscribeToTask(id)

`GET /tasks/:id/subscribe` -- Subscribes to task updates (SSE).

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getTasksApi();
final String id = id_example; // String | Task ID to subscribe to

try {
    api.subscribeToTask(id);
} on DioException catch (e) {
    print('Exception when calling TasksApi->subscribeToTask: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Task ID to subscribe to | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: text/event-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

