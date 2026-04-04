# assistant_api.model.ListTasksResponse

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**nextPageToken** | **String** | A token to retrieve the next page of results. | 
**pageSize** | **int** | The page size used for this response. | 
**tasks** | [**BuiltList&lt;Task&gt;**](Task.md) | Tasks matching the specified criteria. | 
**totalSize** | **int** | Total number of tasks available (before pagination). | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


