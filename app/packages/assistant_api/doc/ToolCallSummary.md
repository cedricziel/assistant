# assistant_api.model.ToolCallSummary

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**arguments** | [**JsonObject**](.md) |  | [optional] 
**name** | **String** | The tool that was called. | 
**result** | **String** | The tool's output, truncated to a reasonable display length. | [optional] 
**status** | **String** | `\"ok\"`, `\"error\"`, or `\"denied\"`. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


