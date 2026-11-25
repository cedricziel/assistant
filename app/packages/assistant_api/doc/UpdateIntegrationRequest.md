# assistant_api.model.UpdateIntegrationRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connectUrl** | **String** | Custom URL integrations only. Changing it deletes the stored credentials, so they cannot be sent to the new host. | [optional] 
**enabled** | **bool** |  | [optional] 
**headerName** | **String** | Custom URL integrations only. | [optional] 
**headerValuePrefix** | **String** | Custom URL integrations only. | [optional] 
**name** | **String** |  | [optional] 
**slug** | **String** |  | [optional] 
**trust** | [**IntegrationTrust**](IntegrationTrust.md) |  | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


