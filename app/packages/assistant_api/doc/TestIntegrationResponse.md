# assistant_api.model.TestIntegrationResponse

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**error** | **String** | Why the connection failed. | [optional] 
**lastConnectedAt** | [**DateTime**](DateTime.md) |  | [optional] 
**ok** | **bool** | Whether a connection was made and its tools listed. | 
**status** | [**IntegrationStatus**](IntegrationStatus.md) | The caller's status after this test. | 
**toolCount** | **int** |  | [optional] 
**toolNames** | **BuiltList&lt;String&gt;** | The tools the server offers, sorted. Empty when the test failed. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


