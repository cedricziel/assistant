# assistant_api.model.MessageSummary

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**attachments** | [**BuiltList&lt;AttachmentMetaResponse&gt;**](AttachmentMetaResponse.md) | Attachments linked to this message. | [optional] 
**branch** | [**BranchInfo**](BranchInfo.md) | This message's place among its versions, present only when it has siblings. On a user message: the edited versions. On an assistant message: the versions of the reply it ends. | [optional] 
**content** | **String** |  | 
**createdAt** | [**DateTime**](DateTime.md) |  | 
**feedback** | [**MessageFeedbackSummary**](MessageFeedbackSummary.md) | The calling user's verdict on this message. Omitted when the caller has not rated it. | [optional] 
**finishReason** | **String** | Why the reply ended early; `\"cancelled\"` for a reply the user stopped. Omitted for every other message. | [optional] 
**id** | **String** |  | 
**model** | **String** | The model that produced this reply. Present on assistant messages saved with it recorded. | [optional] 
**parentId** | **String** | The message this one follows in the conversation tree. Omitted for a root. | [optional] 
**replyBranch** | [**BranchInfo**](BranchInfo.md) | On a user message whose reply has no visible assistant row: the versions of that reply. | [optional] 
**role** | **String** |  | 
**skillName** | **String** | Name of the tool or skill that produced this result (present when `role == \"tool\"`). | [optional] 
**toolCalls** | [**BuiltList&lt;ToolCallSummary&gt;**](ToolCallSummary.md) | Tool calls made in this message (present when `role == \"assistant\"` and the message contains tool invocations). | [optional] 
**ttsAvailable** | **bool** | Whether text-to-speech audio can be synthesised for this message. `true` when a TTS provider is configured and the message is a non-empty assistant reply. | 
**turn** | **int** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


