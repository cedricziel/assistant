# assistant_api.api.ConversationsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**callMcpAppTool**](ConversationsApi.md#callmcpapptool) | **POST** /api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/tools-call | &#x60;POST /api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/tools-call&#x60;
[**cancelTurn**](ConversationsApi.md#cancelturn) | **POST** /api/conversations/{conversation_id}/turns/{turn_id}/cancel | &#x60;POST /api/conversations/{conversation_id}/turns/{turn_id}/cancel&#x60;
[**createConversation**](ConversationsApi.md#createconversation) | **POST** /api/conversations | &#x60;POST /api/conversations&#x60; — create a new conversation.
[**createQuickMessage**](ConversationsApi.md#createquickmessage) | **POST** /api/quick-message | &#x60;POST /api/quick-message&#x60; — create a new conversation, send a message, and return the complete assistant response as a synchronous JSON reply.
[**deleteConversation**](ConversationsApi.md#deleteconversation) | **DELETE** /api/conversations/{id} | &#x60;DELETE /api/conversations/{id}&#x60; — delete a conversation and all its messages.
[**deleteMessageFeedback**](ConversationsApi.md#deletemessagefeedback) | **DELETE** /api/conversations/{id}/messages/{message_id}/feedback | &#x60;DELETE /api/conversations/{id}/messages/{message_id}/feedback&#x60; — clear feedback.
[**editMessage**](ConversationsApi.md#editmessage) | **POST** /api/conversations/{id}/messages/{message_id}/edit | &#x60;POST /api/conversations/{id}/messages/{message_id}/edit&#x60; — replace a user message and stream the new reply.
[**getAudio**](ConversationsApi.md#getaudio) | **GET** /api/audio/{id} | &#x60;GET /api/audio/{id}&#x60; — serve a synthesized audio blob from the in-memory store.
[**getConversation**](ConversationsApi.md#getconversation) | **GET** /api/conversations/{id} | &#x60;GET /api/conversations/{id}&#x60; — get a conversation and its message history.
[**getMessageAudio**](ConversationsApi.md#getmessageaudio) | **GET** /api/messages/{id}/audio | &#x60;GET /api/messages/{id}/audio&#x60; — synthesize TTS audio for an assistant message and return it in the configured &#x60;[tts] format&#x60;.
[**getTurnStatus**](ConversationsApi.md#getturnstatus) | **GET** /api/conversations/{conversation_id}/turns/{turn_id}/status | &#x60;GET /api/conversations/{conversation_id}/turns/{turn_id}/status&#x60;
[**listConversationInteractions**](ConversationsApi.md#listconversationinteractions) | **GET** /api/conversations/{conversation_id}/interactions | &#x60;GET /api/conversations/{conversation_id}/interactions&#x60;
[**listConversations**](ConversationsApi.md#listconversations) | **GET** /api/conversations | &#x60;GET /api/conversations&#x60; — list conversations, newest first.
[**readMcpAppResource**](ConversationsApi.md#readmcpappresource) | **POST** /api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/resources-read | &#x60;POST /api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/resources-read&#x60;
[**regenerateMessage**](ConversationsApi.md#regeneratemessage) | **POST** /api/conversations/{id}/messages/{message_id}/regenerate | &#x60;POST /api/conversations/{id}/messages/{message_id}/regenerate&#x60; — answer a user message again and stream the new reply.
[**respondToInteraction**](ConversationsApi.md#respondtointeraction) | **POST** /api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond | &#x60;POST /api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond&#x60;
[**searchConversations**](ConversationsApi.md#searchconversations) | **GET** /api/conversations/search | Search conversations by title and message text.
[**sendMessage**](ConversationsApi.md#sendmessage) | **POST** /api/conversations/{id}/messages | &#x60;POST /api/conversations/{id}/messages&#x60; — send a message and stream the response.
[**sendVoiceMessage**](ConversationsApi.md#sendvoicemessage) | **POST** /api/conversations/{id}/voice | &#x60;POST /api/conversations/{id}/voice&#x60; — upload audio, keep it as the user message&#39;s attachment, transcribe it, run through the orchestrator, and stream the response as SSE.
[**setMessageFeedback**](ConversationsApi.md#setmessagefeedback) | **PUT** /api/conversations/{id}/messages/{message_id}/feedback | &#x60;PUT /api/conversations/{id}/messages/{message_id}/feedback&#x60; — rate a reply.
[**streamConversations**](ConversationsApi.md#streamconversations) | **GET** /api/conversations/stream | &#x60;GET /api/conversations/stream&#x60; — SSE stream of conversation list changes.
[**streamRunEvents**](ConversationsApi.md#streamrunevents) | **GET** /api/conversations/{id}/runs/{run_id}/events/stream | &#x60;GET /api/conversations/{id}/runs/{run_id}/events/stream&#x60;
[**switchConversationBranch**](ConversationsApi.md#switchconversationbranch) | **POST** /api/conversations/{id}/switch-branch | &#x60;POST /api/conversations/{id}/switch-branch&#x60; — make another version of a message the active one.
[**updateConversation**](ConversationsApi.md#updateconversation) | **PATCH** /api/conversations/{id} | &#x60;PATCH /api/conversations/{id}&#x60; — update a conversation&#39;s title, pin and/or archive state.


# **callMcpAppTool**
> JsonObject callMcpAppTool(conversationId, toolCallId, callMcpAppToolRequest, xSpaceId)

`POST /api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/tools-call`

Call a tool on the server of the app this tool call rendered, as the caller. The tool must be on that server and visible to apps; the call is checked against the permission policy, and one that needs a human (any tool not marked read-only, an untrusted server, or a policy `ask`) runs only with `confirmed: true`. Returns the MCP `CallToolResult`.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String conversationId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation UUID
final String toolCallId = toolCallId_example; // String | Id of the tool call that carries the app
final CallMcpAppToolRequest callMcpAppToolRequest = ; // CallMcpAppToolRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.callMcpAppTool(conversationId, toolCallId, callMcpAppToolRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->callMcpAppTool: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **conversationId** | **String**| Conversation UUID | 
 **toolCallId** | **String**| Id of the tool call that carries the app | 
 **callMcpAppToolRequest** | [**CallMcpAppToolRequest**](CallMcpAppToolRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**JsonObject**](JsonObject.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **cancelTurn**
> TurnStatusResponse cancelTurn(conversationId, turnId, xSpaceId)

`POST /api/conversations/{conversation_id}/turns/{turn_id}/cancel`

Idempotent: triggers `Orchestrator::cancel_turn` if the run belongs to this conversation and a matching token is registered, otherwise no-op. The cancel also covers a turn no worker has claimed yet: its stream ends at once with `agent_error {\"reason\":\"cancelled\",\"user_message_persisted\":false}`. A turn cancelled after it started keeps its streamed text as an assistant message with `finish_reason: \"cancelled\"` (the placeholder `_Stopped._` when nothing streamed), reported as `message_id` on the terminal `agent_error`. Always returns 200 with the current turn status — the cancellation propagates asynchronously through the worker's `tokio::select!`, the SSE stream's terminal `agent_error` event, and the `event_store`. Clients should poll `GET .../status` (or wait for the stream's `agent_error`) to observe the transition.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String conversationId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation UUID
final String turnId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Turn (run) UUID — matches the SSE run_id
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.cancelTurn(conversationId, turnId, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->cancelTurn: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **conversationId** | **String**| Conversation UUID | 
 **turnId** | **String**| Turn (run) UUID — matches the SSE run_id | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**TurnStatusResponse**](TurnStatusResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createConversation**
> ConversationSummary createConversation(createConversationRequest, xSpaceId)

`POST /api/conversations` — create a new conversation.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final CreateConversationRequest createConversationRequest = ; // CreateConversationRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.createConversation(createConversationRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->createConversation: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createConversationRequest** | [**CreateConversationRequest**](CreateConversationRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**ConversationSummary**](ConversationSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createQuickMessage**
> QuickMessageResponse createQuickMessage(quickMessageRequest, xSpaceId)

`POST /api/quick-message` — create a new conversation, send a message, and return the complete assistant response as a synchronous JSON reply.

This endpoint is designed for machine clients (Apple Shortcuts, Siri App Intents, curl, webhooks) that need a single request/response round-trip.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final QuickMessageRequest quickMessageRequest = ; // QuickMessageRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.createQuickMessage(quickMessageRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->createQuickMessage: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **quickMessageRequest** | [**QuickMessageRequest**](QuickMessageRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**QuickMessageResponse**](QuickMessageResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteConversation**
> deleteConversation(id, xSpaceId)

`DELETE /api/conversations/{id}` — delete a conversation and all its messages.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.deleteConversation(id, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->deleteConversation: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteMessageFeedback**
> deleteMessageFeedback(id, messageId, xSpaceId)

`DELETE /api/conversations/{id}/messages/{message_id}/feedback` — clear feedback.

Returns 204 whether or not feedback existed.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final String messageId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Assistant message ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.deleteMessageFeedback(id, messageId, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->deleteMessageFeedback: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **messageId** | **String**| Assistant message ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **editMessage**
> editMessage(id, messageId, editMessageRequest, xSpaceId)

`POST /api/conversations/{id}/messages/{message_id}/edit` — replace a user message and stream the new reply.

Saves the new text as a sibling of `message_id`, carries the original's attachments forward, makes the new branch active and answers it. The original message and everything below it stay stored. The first frame is `run_started{run_id, user_message_id}`, the id the new message is saved under. Returns `409` while a turn is working or another edit or regenerate runs.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final String messageId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | The user message to edit
final EditMessageRequest editMessageRequest = ; // EditMessageRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.editMessage(id, messageId, editMessageRequest, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->editMessage: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **messageId** | **String**| The user message to edit | 
 **editMessageRequest** | [**EditMessageRequest**](EditMessageRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/event-stream, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getAudio**
> getAudio(id, xSpaceId)

`GET /api/audio/{id}` — serve a synthesized audio blob from the in-memory store.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Audio blob ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.getAudio(id, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->getAudio: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Audio blob ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: audio/*

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getConversation**
> ConversationDetail getConversation(id, xSpaceId)

`GET /api/conversations/{id}` — get a conversation and its message history.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.getConversation(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->getConversation: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**ConversationDetail**](ConversationDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getMessageAudio**
> getMessageAudio(id, xSpaceId)

`GET /api/messages/{id}/audio` — synthesize TTS audio for an assistant message and return it in the configured `[tts] format`.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Message ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.getMessageAudio(id, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->getMessageAudio: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Message ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: audio/*, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getTurnStatus**
> TurnStatusResponse getTurnStatus(conversationId, turnId, xSpaceId)

`GET /api/conversations/{conversation_id}/turns/{turn_id}/status`

Returns the authoritative state of the named turn.  Authorisation: the existing conversation-scoped auth middleware applies.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String conversationId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation UUID
final String turnId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Turn (run) UUID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.getTurnStatus(conversationId, turnId, xSpaceId);
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
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**TurnStatusResponse**](TurnStatusResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listConversationInteractions**
> BuiltList<InteractionDetail> listConversationInteractions(conversationId, status, xSpaceId)

`GET /api/conversations/{conversation_id}/interactions`

The interactions of one of the caller's conversations, oldest first, as a bare array. `?status=pending` narrows it to those still waiting, which is how a client that opens a conversation finds a run to attach to.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String conversationId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation UUID
final InteractionStatusView status = ; // InteractionStatusView | Only interactions in this state, such as `pending`.
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.listConversationInteractions(conversationId, status, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->listConversationInteractions: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **conversationId** | **String**| Conversation UUID | 
 **status** | [**InteractionStatusView**](.md)| Only interactions in this state, such as `pending`. | [optional] 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**BuiltList&lt;InteractionDetail&gt;**](InteractionDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listConversations**
> BuiltList<ConversationSummary> listConversations(archived, xSpaceId)

`GET /api/conversations` — list conversations, newest first.

Returns the unarchived conversations, pinned first. With `?archived=true` it returns the archived ones instead, most recently archived first.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final bool archived = true; // bool | `true` lists only archived conversations, newest archive first. Absent or `false` lists the unarchived ones, pinned first.
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.listConversations(archived, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->listConversations: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **archived** | **bool**| `true` lists only archived conversations, newest archive first. Absent or `false` lists the unarchived ones, pinned first. | [optional] 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**BuiltList&lt;ConversationSummary&gt;**](ConversationSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **readMcpAppResource**
> JsonObject readMcpAppResource(conversationId, toolCallId, readMcpAppResourceRequest, xSpaceId)

`POST /api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/resources-read`

Read a resource from the server of the app this tool call rendered, as the caller. Returns the MCP `ReadResourceResult`.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String conversationId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation UUID
final String toolCallId = toolCallId_example; // String | Id of the tool call that carries the app
final ReadMcpAppResourceRequest readMcpAppResourceRequest = ; // ReadMcpAppResourceRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.readMcpAppResource(conversationId, toolCallId, readMcpAppResourceRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->readMcpAppResource: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **conversationId** | **String**| Conversation UUID | 
 **toolCallId** | **String**| Id of the tool call that carries the app | 
 **readMcpAppResourceRequest** | [**ReadMcpAppResourceRequest**](ReadMcpAppResourceRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**JsonObject**](JsonObject.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **regenerateMessage**
> regenerateMessage(id, messageId, xSpaceId)

`POST /api/conversations/{id}/messages/{message_id}/regenerate` — answer a user message again and stream the new reply.

`message_id` is a user message or any row of a reply; a reply row resolves to the user message above it. No user message is saved: the new reply becomes another version of the old one. The first frame is `run_started{run_id, user_message_id}` with the answered message's id. Returns `409` while a turn is working or another edit or regenerate runs.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final String messageId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | A user message, or any row of the reply to regenerate
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.regenerateMessage(id, messageId, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->regenerateMessage: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **messageId** | **String**| A user message, or any row of the reply to regenerate | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: text/event-stream, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **respondToInteraction**
> InteractionDetail respondToInteraction(conversationId, turnId, interactionId, respondToInteractionRequest, xSpaceId)

`POST /api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond`

The turn's owner answers a pending interaction. The answer is validated by the interaction kind's handler. Answering is a compare-and-set on `pending`: repeating the same answer returns `200` with the stored resolution, a different answer after resolution returns `409`, and an expired or cancelled interaction (or one whose turn process stopped waiting) returns `410`.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String conversationId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation UUID
final String turnId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Turn (run) UUID — matches the SSE run_id
final String interactionId = interactionId_example; // String | Interaction id from `interaction_requested`
final RespondToInteractionRequest respondToInteractionRequest = ; // RespondToInteractionRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.respondToInteraction(conversationId, turnId, interactionId, respondToInteractionRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->respondToInteraction: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **conversationId** | **String**| Conversation UUID | 
 **turnId** | **String**| Turn (run) UUID — matches the SSE run_id | 
 **interactionId** | **String**| Interaction id from `interaction_requested` | 
 **respondToInteractionRequest** | [**RespondToInteractionRequest**](RespondToInteractionRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**InteractionDetail**](InteractionDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **searchConversations**
> BuiltList<ConversationSearchSummary> searchConversations(q, limit, xSpaceId)

Search conversations by title and message text.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String q = q_example; // String | Search text, split on whitespace into at most 8 tokens (at most 200 characters). Each token is a prefix of a message word or a substring of the title.
final int limit = 56; // int | Maximum results (default 20, capped at 50).
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.searchConversations(q, limit, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->searchConversations: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **q** | **String**| Search text, split on whitespace into at most 8 tokens (at most 200 characters). Each token is a prefix of a message word or a substring of the title. | [optional] 
 **limit** | **int**| Maximum results (default 20, capped at 50). | [optional] 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**BuiltList&lt;ConversationSearchSummary&gt;**](ConversationSearchSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **sendMessage**
> sendMessage(id, apiSendMessageRequest, xSpaceId)

`POST /api/conversations/{id}/messages` — send a message and stream the response.

The response is a `text/event-stream` (SSE) with two event types: - `event: token` — incremental assistant token (data is plain text) - `event: done`  — final JSON object: `{\"role\":\"assistant\",\"content\":\"...\"}`

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final ApiSendMessageRequest apiSendMessageRequest = ; // ApiSendMessageRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.sendMessage(id, apiSendMessageRequest, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->sendMessage: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **apiSendMessageRequest** | [**ApiSendMessageRequest**](ApiSendMessageRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/event-stream, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **sendVoiceMessage**
> sendVoiceMessage(id, audio, xSpaceId)

`POST /api/conversations/{id}/voice` — upload audio, keep it as the user message's attachment, transcribe it, run through the orchestrator, and stream the response as SSE.

The message text is the transcript, or `[Voice message]` when no transcription provider is configured or transcription fails.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final BuiltList<int> audio = ; // BuiltList<int> | Raw audio bytes (opus/aac/webm/wav …).
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.sendVoiceMessage(id, audio, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->sendVoiceMessage: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **audio** | [**BuiltList&lt;int&gt;**](int.md)| Raw audio bytes (opus/aac/webm/wav …). | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: multipart/form-data
 - **Accept**: text/event-stream, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **setMessageFeedback**
> MessageFeedbackResponse setMessageFeedback(id, messageId, setMessageFeedbackRequest, xSpaceId)

`PUT /api/conversations/{id}/messages/{message_id}/feedback` — rate a reply.

Replaces the caller's previous feedback on the message in full.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final String messageId = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Assistant message ID
final SetMessageFeedbackRequest setMessageFeedbackRequest = ; // SetMessageFeedbackRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.setMessageFeedback(id, messageId, setMessageFeedbackRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->setMessageFeedback: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **messageId** | **String**| Assistant message ID | 
 **setMessageFeedbackRequest** | [**SetMessageFeedbackRequest**](SetMessageFeedbackRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**MessageFeedbackResponse**](MessageFeedbackResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **streamConversations**
> streamConversations(agentId, xSpaceId)

`GET /api/conversations/stream` — SSE stream of conversation list changes.

Sends an initial `snapshot` event with the full conversation list, then pushes `upserted` and `deleted` delta events as conversations change.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String agentId = agentId_example; // String | Filter events to a single agent. If omitted, events for all agents are streamed.
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.streamConversations(agentId, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->streamConversations: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **agentId** | **String**| Filter events to a single agent. If omitted, events for all agents are streamed. | [optional] 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: text/event-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **streamRunEvents**
> streamRunEvents(id, runId, since, xSpaceId)

`GET /api/conversations/{id}/runs/{run_id}/events/stream`

Replays stored events from `?since` (default 0), then tails live events if the run is still active. Closes when the run finishes, not at the `done` or `agent_error` frame: events such as `suggestions` follow `done`. A client that disconnects stops the tail at once.  Returns `404` if no events exist for `run_id` and the run is not active (never started, unknown, or pruned after the TTL elapsed).  After `done` the stream may carry one `suggestions` frame (`SseSuggestionsEvent`) with follow-up prompts for the finished turn; it then closes when the run finishes.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = id_example; // String | Conversation UUID
final String runId = runId_example; // String | Run UUID from run_started event
final int since = 789; // int | Replay from this sequence number (default 0)
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.streamRunEvents(id, runId, since, xSpaceId);
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
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: text/event-stream, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **switchConversationBranch**
> ConversationDetail switchConversationBranch(id, switchBranchRequest, xSpaceId)

`POST /api/conversations/{id}/switch-branch` — make another version of a message the active one.

Lands on the most recently created leaf below `message_id`. Does not change `updated_at`. While a turn is working it returns `409`; while a turn only waits for an approval or an answer, that wait is cancelled first.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final SwitchBranchRequest switchBranchRequest = ; // SwitchBranchRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.switchConversationBranch(id, switchBranchRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ConversationsApi->switchConversationBranch: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **switchBranchRequest** | [**SwitchBranchRequest**](SwitchBranchRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**ConversationDetail**](ConversationDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateConversation**
> ConversationSummary updateConversation(id, updateConversationRequest, xSpaceId)

`PATCH /api/conversations/{id}` — update a conversation's title, pin and/or archive state.

None of these changes moves the conversation in the list: only new messages bump `updated_at`. Archiving clears the pin; pinning an archived conversation unarchives it.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getConversationsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final UpdateConversationRequest updateConversationRequest = ; // UpdateConversationRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.updateConversation(id, updateConversationRequest, xSpaceId);
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
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**ConversationSummary**](ConversationSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

