# assistant_api.model.SseDoneEvent

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**attachmentIds** | **BuiltList&lt;String&gt;** | Attachments the reply produced. Omitted when there are none. | [optional] 
**content** | **String** | The full reply text. | 
**messageId** | **String** | The saved assistant message. Absent when the turn saved none. | [optional] 
**role** | **String** | Always `assistant`. | 
**timing** | [**MessageTimingSummary**](MessageTimingSummary.md) | How long the turn took. Absent when none was measured. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


