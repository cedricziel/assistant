# assistant_api.model.MessageSummary

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**content** | **String** |  | 
**createdAt** | [**DateTime**](DateTime.md) |  | 
**id** | **String** |  | 
**role** | **String** |  | 
**skillName** | **String** | Name of the tool or skill that produced this result (present when `role == \"tool\"`). | [optional] 
**toolCalls** | **BuiltList&lt;String&gt;** | Tool names called in this message (present when `role == \"assistant\"` and the message contains tool invocations). | [optional] 
**turn** | **int** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


