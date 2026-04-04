# assistant_api.model.SendMessageConfiguration

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**acceptedOutputModes** | **BuiltList&lt;String&gt;** | Media types the client accepts for response parts. | [optional] 
**blocking** | **bool** | If true, wait until the task reaches a terminal or interrupted state. | [optional] 
**historyLength** | **int** | Max number of recent messages to include in the response. | [optional] 
**pushNotificationConfig** | [**PushNotificationConfig**](PushNotificationConfig.md) | Push notification configuration for task updates. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


