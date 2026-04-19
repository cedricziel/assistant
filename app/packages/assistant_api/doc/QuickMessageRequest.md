# assistant_api.model.QuickMessageRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**context** | **String** | Optional context text to include with the message (e.g. clipboard contents, file text).  Prepended to the user message for the LLM. | [optional] 
**message** | **String** | The message text to send to the assistant. | 
**personaId** | **String** | Optional persona ID to route the message to a specific persona. When absent, the server's active persona is used. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


