# assistant_api.model.SetMessageFeedbackRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**comment** | **String** | Optional free-text reason. Trimmed; an empty result means no comment. At most 2000 Unicode scalar values. Omitting it clears a stored comment. | [optional] 
**rating** | [**FeedbackRating**](FeedbackRating.md) | Thumbs up or down. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


