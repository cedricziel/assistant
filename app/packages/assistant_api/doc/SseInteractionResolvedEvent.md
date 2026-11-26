# assistant_api.model.SseInteractionResolvedEvent

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**interactionId** | **String** |  | 
**kind** | **String** |  | 
**outcome** | [**InteractionOutcomeView**](InteractionOutcomeView.md) |  | 
**resolvedAt** | [**DateTime**](DateTime.md) |  | 
**response** | [**JsonObject**](.md) | The owner's answer. Only when `outcome` is `resolved`. | [optional] 
**ruleId** | **String** | Id of the permission rule an \"always\" answer saved. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


