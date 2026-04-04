# assistant_api.model.SendMessageRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**configuration** | [**SendMessageConfiguration**](SendMessageConfiguration.md) | Configuration for the send request. | [optional] 
**message** | [**Message**](Message.md) | The message to send to the agent. | 
**metadata** | [**BuiltMap&lt;String, JsonObject&gt;**](JsonObject.md) | Additional context or parameters. | [optional] 
**tenant** | **String** | Optional tenant ID. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


