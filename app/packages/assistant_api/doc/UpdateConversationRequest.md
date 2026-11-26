# assistant_api.model.UpdateConversationRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**archived** | **bool** | `true` archives the conversation (clearing its pin), `false` unarchives it. Pinning an archived conversation also unarchives it. Combining `pinned: true` with `archived: true` is rejected. | [optional] 
**pinned** | **bool** | `true` pins the conversation, `false` unpins it. | [optional] 
**title** | **String** | New title. Trimmed; must be 1 to 200 characters after trimming. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


