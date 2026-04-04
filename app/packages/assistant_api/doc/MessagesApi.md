# assistant_api.api.MessagesApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**sendMessage**](MessagesApi.md#sendmessage) | **POST** /message/send | &#x60;POST /message/send&#x60; -- Sends a message to the agent (unary).
[**sendMessageStreaming**](MessagesApi.md#sendmessagestreaming) | **POST** /message/stream | &#x60;POST /message/stream&#x60; -- Sends a message with streaming response (SSE).


# **sendMessage**
> SendMessageResponse sendMessage(sendMessageRequest)

`POST /message/send` -- Sends a message to the agent (unary).

Creates a task, records the user message, transitions to Working, produces an agent reply, and returns the final task state.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getMessagesApi();
final SendMessageRequest sendMessageRequest = ; // SendMessageRequest | 

try {
    final response = api.sendMessage(sendMessageRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling MessagesApi->sendMessage: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **sendMessageRequest** | [**SendMessageRequest**](SendMessageRequest.md)|  | 

### Return type

[**SendMessageResponse**](SendMessageResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **sendMessageStreaming**
> sendMessageStreaming(sendMessageRequest)

`POST /message/stream` -- Sends a message with streaming response (SSE).

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getMessagesApi();
final SendMessageRequest sendMessageRequest = ; // SendMessageRequest | 

try {
    api.sendMessageStreaming(sendMessageRequest);
} on DioException catch (e) {
    print('Exception when calling MessagesApi->sendMessageStreaming: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **sendMessageRequest** | [**SendMessageRequest**](SendMessageRequest.md)|  | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/event-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

