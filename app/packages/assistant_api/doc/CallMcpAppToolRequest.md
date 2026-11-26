# assistant_api.model.CallMcpAppToolRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**arguments** | [**JsonObject**](.md) | The tool's arguments. | [optional] 
**confirmed** | **bool** | The user confirmed this call. Required for a tool without `readOnlyHint: true`, and for any call the permission policy leaves to a human; without it such a call is refused with `409`. | [optional] 
**name** | **String** | The tool's name on the app's server. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


