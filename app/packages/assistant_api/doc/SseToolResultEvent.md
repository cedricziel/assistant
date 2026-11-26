# assistant_api.model.SseToolResultEvent

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**app** | [**McpAppView**](McpAppView.md) | The MCP App the tool declared for its result. Absent for tools without one, and on subagent tool results. | [optional] 
**arguments** | [**JsonObject**](.md) | The arguments passed to the tool, as a JSON object. | [optional] 
**data** | [**JsonObject**](.md) | Structured output of the tool, a JSON object with a `kind` tag. Absent when the tool returned text only or the data exceeded the size limit. Each `kind` is defined by a JSON Schema in `schemas/tool-results/`. | [optional] 
**result** | **String** | The tool's output or error message. | [optional] 
**status** | **String** | Whether the tool call succeeded or failed. | 
**toolCallId** | **String** | Identifier of the tool call this result answers, when the provider supplied one. | [optional] 
**toolName** | **String** | Name of the tool that was called. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


