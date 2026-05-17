# assistant_api.api.ConversationsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createConversation**](ConversationsApi.md#createconversation) | **POST** /api/conversations | &#x60;POST /api/conversations&#x60; — create a new conversation.
[**createQuickMessage**](ConversationsApi.md#createquickmessage) | **POST** /api/quick-message | &#x60;POST /api/quick-message&#x60; — create a new conversation, send a message, and return the complete assistant response as a synchronous JSON reply.
[**deleteConversation**](ConversationsApi.md#deleteconversation) | **DELETE** /api/conversations/{id} | &#x60;DELETE /api/conversations/{id}&#x60; — delete a conversation and all its messages.
[**getAudio**](ConversationsApi.md#getaudio) | **GET** /api/audio/{id} | &#x60;GET /api/audio/{id}&#x60; — serve a synthesized audio blob from the in-memory store.
[**getConversation**](ConversationsApi.md#getconversation) | **GET** /api/conversations/{id} | &#x60;GET /api/conversations/{id}&#x60; — get a conversation and its message history.
[**getMessageAudio**](ConversationsApi.md#getmessageaudio) | **GET** /api/messages/{id}/audio | &#x60;GET /api/messages/{id}/audio&#x60; — synthesize TTS audio for an assistant message and return it as &#x60;audio/mpeg&#x60;.
[**getTurnStatus**](ConversationsApi.md#getturnstatus) | **GET** /api/conversations/{conversation_id}/turns/{turn_id}/status | &#x60;GET /api/conversations/{conversation_id}/turns/{turn_id}/status&#x60;
[**listConversations**](ConversationsApi.md#listconversations) | **GET** /api/conversations | &#x60;GET /api/conversations&#x60; — list all conversations, newest first.
[**sendMessage**](ConversationsApi.md#sendmessage) | **POST** /api/conversations/{id}/messages | &#x60;POST /api/conversations/{id}/messages&#x60; — send a message and stream the response.
[**sendVoiceMessage**](ConversationsApi.md#sendvoicemessage) | **POST** /api/conversations/{id}/voice | &#x60;POST /api/conversations/{id}/voice&#x60; — upload audio, transcribe it, run through the orchestrator, and stream the response as SSE.
[**streamConversations**](ConversationsApi.md#streamconversations) | **GET** /api/conversations/stream | &#x60;GET /api/conversations/stream&#x60; — SSE stream of conversation list changes.
[**streamRunEvents**](ConversationsApi.md#streamrunevents) | **GET** /api/conversations/{id}/runs/{run_id}/events/stream | &#x60;GET /api/conversations/{id}/runs/{run_id}/events/stream&#x60;
[**updateConversation**](ConversationsApi.md#updateconversation) | **PATCH** /api/conversations/{id} | &#x60;PATCH /api/conversations/{id}&#x60; — update a conversation&#39;s title.


# **createConversation**
> ConversationSummary createConversation(createConversationRequest)

`POST /api/conversations` — create a new conversation.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final CreateConversationRequest createConversationRequest = ; // CreateConversationRequest | 

try {
    final response = api.createConversation(createConversationRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->createConversation: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createConversationRequest** | [**CreateConversationRequest**](CreateConversationRequest.md)|  | 

### Return type

[**ConversationSummary**](ConversationSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createQuickMessage**
> QuickMessageResponse createQuickMessage(quickMessageRequest)

`POST /api/quick-message` — create a new conversation, send a message, and return the complete assistant response as a synchronous JSON reply.

This endpoint is designed for machine clients (Apple Shortcuts, Siri App Intents, curl, webhooks) that need a single request/response round-trip.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final QuickMessageRequest quickMessageRequest = ; // QuickMessageRequest | 

try {
    final response = api.createQuickMessage(quickMessageRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->createQuickMessage: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **quickMessageRequest** | [**QuickMessageRequest**](QuickMessageRequest.md)|  | 

### Return type

[**QuickMessageResponse**](QuickMessageResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteConversation**
> deleteConversation(id)

`DELETE /api/conversations/{id}` — delete a conversation and all its messages.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID

try {
    api.deleteConversation(id);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->deleteConversation: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getAudio**
> getAudio(id)

`GET /api/audio/{id}` — serve a synthesized audio blob from the in-memory store.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Audio blob ID

try {
    api.getAudio(id);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->getAudio: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Audio blob ID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: audio/mpeg

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getConversation**
> ConversationDetail getConversation(id)

`GET /api/conversations/{id}` — get a conversation and its message history.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID

try {
    final response = api.getConversation(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->getConversation: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 

### Return type

[**ConversationDetail**](ConversationDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getMessageAudio**
> getMessageAudio(id)

`GET /api/messages/{id}/audio` — synthesize TTS audio for an assistant message and return it as `audio/mpeg`.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Message ID

try {
    api.getMessageAudio(id);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->getMessageAudio: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Message ID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: audio/mpeg

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getTurnStatus**
> TurnStatusResponse getTurnStatus(conversationId, turnId)

`GET /api/conversations/{conversation_id}/turns/{turn_id}/status`

Returns the authoritative state of the named turn.  Authorisation: the existing conversation-scoped auth middleware applies.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String conversationId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation UUID
final String turnId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Turn (run) UUID

try {
    final response = api.getTurnStatus(conversationId, turnId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->getTurnStatus: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **conversationId** | **String**| Conversation UUID | 
 **turnId** | **String**| Turn (run) UUID | 

### Return type

[**TurnStatusResponse**](TurnStatusResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listConversations**
> BuiltList<ConversationSummary> listConversations()

`GET /api/conversations` — list all conversations, newest first.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();

try {
    final response = api.listConversations();
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->listConversations: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**BuiltList&lt;ConversationSummary&gt;**](ConversationSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **sendMessage**
> sendMessage(id, apiSendMessageRequest)

`POST /api/conversations/{id}/messages` — send a message and stream the response.

The response is a `text/event-stream` (SSE) with two event types: - `event: token` — incremental assistant token (data is plain text) - `event: done`  — final JSON object: `{\"role\":\"assistant\",\"content\":\"...\"}`

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final ApiSendMessageRequest apiSendMessageRequest = ; // ApiSendMessageRequest | 

try {
    api.sendMessage(id, apiSendMessageRequest);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->sendMessage: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **apiSendMessageRequest** | [**ApiSendMessageRequest**](ApiSendMessageRequest.md)|  | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/event-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **sendVoiceMessage**
> sendVoiceMessage(id, audio)

`POST /api/conversations/{id}/voice` — upload audio, transcribe it, run through the orchestrator, and stream the response as SSE.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final BuiltList<int> audio = ; // BuiltList<int> | Raw audio bytes (opus/aac/webm/wav …).

try {
    api.sendVoiceMessage(id, audio);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->sendVoiceMessage: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **audio** | [**BuiltList&lt;int&gt;**](int.md)| Raw audio bytes (opus/aac/webm/wav …). | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: multipart/form-data
 - **Accept**: text/event-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **streamConversations**
> streamConversations(agentId)

`GET /api/conversations/stream` — SSE stream of conversation list changes.

Sends an initial `snapshot` event with the full conversation list, then pushes `upserted` and `deleted` delta events as conversations change.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String agentId = agentId_example; // String | Filter events to a single agent. If omitted, events for all agents are streamed.

try {
    api.streamConversations(agentId);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->streamConversations: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **agentId** | **String**| Filter events to a single agent. If omitted, events for all agents are streamed. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: text/event-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **streamRunEvents**
> streamRunEvents(id, runId, since)

`GET /api/conversations/{id}/runs/{run_id}/events/stream`

Replays stored events from `?since` (default 0), then tails live events if the run is still active.  Closes automatically when the `done` or `error` event is reached.  Returns: - `404` if no events exist for `run_id` (run never started or unknown) - `410` if the run existed but all events have been pruned (TTL elapsed)

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = id_example; // String | Conversation UUID
final String runId = runId_example; // String | Run UUID from run_started event
final int since = 789; // int | Replay from this sequence number (default 0)

try {
    api.streamRunEvents(id, runId, since);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->streamRunEvents: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation UUID | 
 **runId** | **String**| Run UUID from run_started event | 
 **since** | **int**| Replay from this sequence number (default 0) | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: text/event-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateConversation**
> ConversationSummary updateConversation(id, updateConversationRequest)

`PATCH /api/conversations/{id}` — update a conversation's title.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final UpdateConversationRequest updateConversationRequest = ; // UpdateConversationRequest | 

try {
    final response = api.updateConversation(id, updateConversationRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->updateConversation: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **updateConversationRequest** | [**UpdateConversationRequest**](UpdateConversationRequest.md)|  | 

### Return type

[**ConversationSummary**](ConversationSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

