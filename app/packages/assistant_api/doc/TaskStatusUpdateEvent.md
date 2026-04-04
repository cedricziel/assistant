# assistant_api.model.TaskStatusUpdateEvent

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**contextId** | **String** | The ID of the context that the task belongs to. | 
**metadata** | [**BuiltMap&lt;String, JsonObject&gt;**](JsonObject.md) | Metadata associated with the task update. | [optional] 
**status** | [**TaskStatus**](TaskStatus.md) | The new status of the task. | 
**taskId** | **String** | The ID of the task that has changed. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


