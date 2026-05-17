# assistant_api.model.TurnStatusResponse

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**conversationId** | **String** |  | 
**lastEventAt** | **String** | Wall-clock timestamp of the most recent SSE event recorded for the turn, in RFC 3339 format. `null` when [`state`] is `unknown`. | [optional] 
**lastEventKind** | **String** | Kind of the most recent SSE event (`run_started`, `token`, `status`, `thinking`, `tool_result`, `agent_error`, `done`, etc.). `null` when [`state`] is `unknown`. | [optional] 
**state** | [**TurnState**](TurnState.md) |  | 
**turnId** | **String** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


