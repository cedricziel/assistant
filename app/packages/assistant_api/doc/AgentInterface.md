# assistant_api.model.AgentInterface

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**protocolBinding** | **String** | The protocol binding (e.g., \"JSONRPC\", \"GRPC\", \"HTTP+JSON\"). | 
**protocolVersion** | **String** | The version of the A2A protocol this interface exposes (e.g., \"1.0\"). | 
**tenant** | **String** | Tenant ID to be used in the request when calling the agent. | [optional] 
**url** | **String** | The URL where this interface is available. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


