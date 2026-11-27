# assistant_api.model.RespondToInteractionRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**decision** | [**ToolDecision**](ToolDecision.md) |  | 
**kind** | **String** |  | 
**answers** | [**BuiltList&lt;QuestionAnswer&gt;**](QuestionAnswer.md) |  | 
**action** | [**JsonObject**](.md) | The action object: `name` (a Button action the surface declares), plus `sourceComponentId` and `context` when the client has them. | 
**dataModel** | [**JsonObject**](.md) | The surface's data model at the time of the action. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


