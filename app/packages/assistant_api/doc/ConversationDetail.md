# assistant_api.model.ConversationDetail

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**createdAt** | [**DateTime**](DateTime.md) |  | 
**id** | **String** |  | 
**interactions** | [**BuiltList&lt;InteractionDetail&gt;**](InteractionDetail.md) | Every interaction of the conversation (tool approvals, questions), in creation order. Includes those of runs on inactive branches; the client decides what to show. | 
**messages** | [**BuiltList&lt;MessageSummary&gt;**](MessageSummary.md) |  | 
**surfaces** | [**BuiltList&lt;SurfaceSummary&gt;**](SurfaceSummary.md) | Generative UI surfaces anchored on the returned messages. | 
**title** | **String** |  | 
**updatedAt** | [**DateTime**](DateTime.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


