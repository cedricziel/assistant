# assistant_api.model.InteractionDetail

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**conversationId** | **String** |  | 
**createdAt** | [**DateTime**](DateTime.md) |  | 
**expiresAt** | [**DateTime**](DateTime.md) |  | 
**interactionId** | **String** |  | 
**kind** | **String** | `tool_approval`, `question`, `surface_action`, or a kind a newer server added. | 
**rendersCard** | **bool** | Whether the kind renders a card in the timeline. | 
**request** | [**JsonObject**](.md) | The kind-specific request (for `tool_approval`: `tool_name`, `arguments`, `arguments_truncated`, `reason`, `options`, `suggested_rule`; for `question`: `questions`). | 
**resolvedAt** | [**DateTime**](DateTime.md) |  | [optional] 
**response** | [**JsonObject**](.md) | The owner's answer. Present once `status` is `resolved`. | [optional] 
**ruleId** | **String** | Id of the permission rule an \"always\" answer saved. | [optional] 
**runId** | **String** | The run (turn) that asked. | 
**status** | [**InteractionStatusView**](InteractionStatusView.md) |  | 
**toolCallId** | **String** | Provider id of the tool call this interaction belongs to, if any. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


