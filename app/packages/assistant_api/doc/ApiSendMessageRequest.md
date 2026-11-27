# assistant_api.model.ApiSendMessageRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**attachmentIds** | **BuiltList&lt;String&gt;** | Optional attachment IDs to include with this message. | [optional] 
**message** | **String** | The message text to send to the assistant. Empty when `surface_action` carries the turn. | 
**surfaceAction** | [**SurfaceActionRequest**](SurfaceActionRequest.md) | Answers an active surface after its turn ended. When present, `message` must be empty; the server builds the visible text. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


