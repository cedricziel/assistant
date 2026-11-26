# assistant_api.model.ConversationSearchSummary

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**createdAt** | [**DateTime**](DateTime.md) |  | 
**id** | **String** |  | 
**matches** | [**BuiltList&lt;MessageSearchMatch&gt;**](MessageSearchMatch.md) | Up to 3 matching messages, best first; empty when only the title matched. | 
**pinnedAt** | [**DateTime**](DateTime.md) | When the conversation was pinned; absent when it is not pinned. | [optional] 
**title** | **String** |  | 
**titleSegments** | [**BuiltList&lt;SearchSegment&gt;**](SearchSegment.md) | The title split into highlighted and plain runs. | 
**updatedAt** | [**DateTime**](DateTime.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


