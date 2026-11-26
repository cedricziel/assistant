# assistant_api.model.ToolCallSummary

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**arguments** | [**JsonObject**](.md) |  | [optional] 
**data** | [**JsonObject**](.md) | Structured output of the tool, a JSON object with a `kind` tag. Absent when the tool returned text only, the data exceeded the size limit, or the call predates structured data. Each `kind` is defined by a JSON Schema in `schemas/tool-results/`. | [optional] 
**name** | **String** | The tool that was called. | 
**result** | **String** | The tool's output, truncated to a reasonable display length. | [optional] 
**status** | **String** | `\"ok\"`, `\"error\"`, or `\"denied\"`. | 
**toolCallId** | **String** | Provider-assigned id of the call, when the provider supplied one. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


