# assistant_api.model.SseInteractionRequestedEvent

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**createdAt** | [**DateTime**](DateTime.md) |  | 
**expiresAt** | [**DateTime**](DateTime.md) |  | 
**interactionId** | **String** |  | 
**kind** | **String** | `tool_approval`, `question`, `surface_action`, or a kind a newer server added. | 
**rendersCard** | **bool** | Whether the kind renders a card in the timeline. | 
**toolCallId** | **String** | Provider id of the tool call this belongs to, when there is one. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


