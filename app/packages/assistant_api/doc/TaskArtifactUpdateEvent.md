# assistant_api.model.TaskArtifactUpdateEvent

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**append** | **bool** | If true, append content to a previously sent artifact with the same ID. | [optional] 
**artifact** | [**Artifact**](Artifact.md) | The artifact that was generated or updated. | 
**contextId** | **String** | The ID of the context that this task belongs to. | 
**lastChunk** | **bool** | If true, this is the final chunk of the artifact. | [optional] 
**metadata** | [**BuiltMap&lt;String, JsonObject&gt;**](JsonObject.md) | Metadata associated with the artifact update. | [optional] 
**taskId** | **String** | The ID of the task for this artifact. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


