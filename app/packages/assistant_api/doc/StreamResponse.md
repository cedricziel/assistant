# assistant_api.model.StreamResponse

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**artifactUpdate** | [**TaskArtifactUpdateEvent**](TaskArtifactUpdateEvent.md) | An event indicating a task artifact update. | [optional] 
**message** | [**Message**](Message.md) | A Message object containing a message from the agent. | [optional] 
**statusUpdate** | [**TaskStatusUpdateEvent**](TaskStatusUpdateEvent.md) | An event indicating a task status update. | [optional] 
**task** | [**Task**](Task.md) | A Task object containing the current state of the task. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


