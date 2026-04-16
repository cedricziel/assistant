# assistant_api.api.MessagesApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**a2aSendMessage**](MessagesApi.md#a2asendmessage) | **POST** /message/send | &#x60;POST /message/send&#x60; -- Sends a message to the agent (unary).
[**a2aSendMessageStreaming**](MessagesApi.md#a2asendmessagestreaming) | **POST** /message/stream | &#x60;POST /message/stream&#x60; -- Sends a message with streaming response (SSE).


# **a2aSendMessage**
> SendMessageResponse a2aSendMessage(sendMessageRequest)

`POST /message/send` -- Sends a message to the agent (unary).

Creates a task, records the user message, transitions to Working, produces an agent reply, and returns the final task state.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getMessagesApi();
final SendMessageRequest sendMessageRequest = ; // SendMessageRequest | 

try {
    final response = api.a2aSendMessage(sendMessageRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling MessagesApi->a2aSendMessage: $e\n');
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

# **a2aSendMessageStreaming**
> StreamResponse a2aSendMessageStreaming(sendMessageRequest)

`POST /message/stream` -- Sends a message with streaming response (SSE).

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getMessagesApi();
final SendMessageRequest sendMessageRequest = ; // SendMessageRequest | 

try {
    final response = api.a2aSendMessageStreaming(sendMessageRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling MessagesApi->a2aSendMessageStreaming: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **sendMessageRequest** | [**SendMessageRequest**](SendMessageRequest.md)|  | 

### Return type

[**StreamResponse**](StreamResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/event-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

