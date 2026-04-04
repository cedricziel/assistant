# assistant_api.model.ListTasksRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**contextId** | **String** | Filter tasks by context ID. | [optional] 
**historyLength** | **int** | Max number of messages to include in each task's history. | [optional] 
**includeArtifacts** | **bool** | Whether to include artifacts in returned tasks. | [optional] 
**pageSize** | **int** | Max number of tasks to return (1..=100, default 50). | [optional] 
**pageToken** | **String** | Page token from a previous `ListTasks` call. | [optional] 
**status** | [**TaskState**](TaskState.md) | Filter tasks by current status state. | [optional] 
**statusTimestampAfter** | [**DateTime**](DateTime.md) | Filter tasks with status updated after this timestamp. | [optional] 
**tenant** | **String** | Tenant ID. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


