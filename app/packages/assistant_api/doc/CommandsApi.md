# assistant_api.api.CommandsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**executeCommand**](CommandsApi.md#executecommand) | **POST** /api/conversations/{id}/command | &#x60;POST /api/conversations/{id}/command&#x60; — execute a slash command.
[**listCommands**](CommandsApi.md#listcommands) | **GET** /api/commands | &#x60;GET /api/commands&#x60; — list all registered commands.
[**listConversationEvents**](CommandsApi.md#listconversationevents) | **GET** /api/conversations/{id}/events | &#x60;GET /api/conversations/{id}/events&#x60; — list command events for the timeline.


# **executeCommand**
> CommandEventResponse executeCommand(id, executeCommandRequest)

`POST /api/conversations/{id}/command` — execute a slash command.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getCommandsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final ExecuteCommandRequest executeCommandRequest = ; // ExecuteCommandRequest | 

try {
    final response = api.executeCommand(id, executeCommandRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling CommandsApi->executeCommand: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **executeCommandRequest** | [**ExecuteCommandRequest**](ExecuteCommandRequest.md)|  | 

### Return type

[**CommandEventResponse**](CommandEventResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listCommands**
> BuiltList<CommandDefResponse> listCommands()

`GET /api/commands` — list all registered commands.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getCommandsApi();

try {
    final response = api.listCommands();
    print(response);
} on DioException catch (e) {
    print('Exception when calling CommandsApi->listCommands: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**BuiltList&lt;CommandDefResponse&gt;**](CommandDefResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listConversationEvents**
> BuiltList<CommandEventResponse> listConversationEvents(id)

`GET /api/conversations/{id}/events` — list command events for the timeline.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getCommandsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID

try {
    final response = api.listConversationEvents(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling CommandsApi->listConversationEvents: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 

### Return type

[**BuiltList&lt;CommandEventResponse&gt;**](CommandEventResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

