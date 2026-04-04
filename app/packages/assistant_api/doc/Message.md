# assistant_api.model.Message

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**contextId** | **String** | The context id of the message. | [optional] 
**extensions** | **BuiltList&lt;String&gt;** | The URIs of extensions present or contributing to this message. | [optional] 
**messageId** | **String** | Unique identifier (UUID) of the message, created by the message creator. | 
**metadata** | [**BuiltMap&lt;String, JsonObject&gt;**](JsonObject.md) | Any metadata to provide along with the message. | [optional] 
**parts** | [**BuiltList&lt;ModelPart&gt;**](ModelPart.md) | Content parts of the message. | 
**referenceTaskIds** | **BuiltList&lt;String&gt;** | A list of task IDs that this message references for additional context. | [optional] 
**role** | [**Role**](Role.md) | Identifies the sender of the message. | 
**taskId** | **String** | The task id of the message. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


