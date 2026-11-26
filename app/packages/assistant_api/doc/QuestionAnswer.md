# assistant_api.model.QuestionAnswer

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**questionId** | **String** | Id of the question being answered. | 
**selectedOptionIds** | **BuiltList&lt;String&gt;** | Ids of the chosen options. Empty when the answer is free text only. | [optional] 
**text** | **String** | Free-text answer, at most 4000 characters, where the question allows it. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


