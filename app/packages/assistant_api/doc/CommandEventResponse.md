# assistant_api.model.CommandEventResponse

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**ackText** | **String** | Acknowledgement text returned by the command. | [optional] 
**command** | **String** | Command that was executed. | 
**createdAt** | [**DateTime**](DateTime.md) | When the event was created. | 
**eventType** | **String** | Event type (always `\"command\"` for now). | 
**id** | **String** | Unique event ID. | 
**payload** | [**JsonObject**](.md) |  | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


