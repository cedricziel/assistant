# assistant_api.model.SurfaceSummary

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**actionMessageId** | **String** | The user message that carried the submitting action. | [optional] 
**anchorCallIndex** | **int** | Index of the creating call inside that message's tool calls. | 
**anchorMessageId** | **String** | The tool-call message the surface hangs off. | 
**answerable** | **bool** | Whether the user can still answer the surface: it is `active` and no user message follows its anchor on the returned path. | 
**createdAt** | [**DateTime**](DateTime.md) |  | 
**fallbackText** | **String** |  | 
**messages** | [**BuiltList&lt;JsonObject&gt;**](JsonObject.md) | A single synthesized A2UI `createSurface` holding the current components and data model. | 
**status** | **String** | `\"active\"` or `\"submitted\"`. | 
**submittedAction** | [**JsonObject**](.md) | The action that submitted the surface. | [optional] 
**surfaceId** | **String** |  | 
**title** | **String** |  | 
**toolCallId** | **String** | Provider id of the creating tool call, when the provider supplied one. | [optional] 
**turn** | **int** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


