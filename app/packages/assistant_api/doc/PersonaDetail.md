# assistant_api.model.PersonaDetail

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**canEditStarterPrompts** | **bool** | True when the caller may `PATCH` this Persona's starter prompts. | 
**createdAt** | **String** |  | 
**files** | [**BuiltList&lt;PersonaFileSlot&gt;**](PersonaFileSlot.md) |  | 
**id** | **String** |  | 
**name** | **String** |  | 
**skillAccessMode** | **String** |  | 
**starterPrompts** | **BuiltList&lt;String&gt;** | Up to 4 prompts shown as suggestions in an empty chat. Always present. | 
**turnTimeoutSecs** | **int** |  | [optional] 
**updatedAt** | **String** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


