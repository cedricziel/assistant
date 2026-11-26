# assistant_api.model.BranchInfo

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**count** | **int** | How many versions there are (always at least 2). | 
**index** | **int** | Zero-based position of the active version, in creation order. | 
**siblingIds** | **BuiltList&lt;String&gt;** | The first message of every version, in creation order. Switching to any of them selects that version. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


