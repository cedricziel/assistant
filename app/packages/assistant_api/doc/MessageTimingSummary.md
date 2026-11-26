# assistant_api.model.MessageTimingSummary

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**decodeMs** | **int** | Time the model spent generating tokens, in milliseconds. | [optional] 
**outputTokens** | **int** | Output tokens summed over every model call in the turn. | [optional] 
**timeToFirstTokenMs** | **int** | Turn start to the first visible answer token, in milliseconds. | [optional] 
**totalMs** | **int** | Turn start to just before the final answer was saved, in milliseconds. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


