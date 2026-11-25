# assistant_api.model.CatalogEntrySummary

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**auth** | [**CatalogAuthResponse**](CatalogAuthResponse.md) |  | 
**categories** | **BuiltList&lt;String&gt;** |  | 
**connectUrl** | **String** | HTTPS endpoint of the remote MCP server. | 
**description** | **String** |  | 
**domain** | **String** |  | 
**iconUrl** | **String** |  | [optional] 
**id** | **String** | Upstream catalog id, e.g. `curated/linear-app-mcp`. Contains a slash. | 
**installableLayers** | [**BuiltList&lt;IntegrationLayer&gt;**](IntegrationLayer.md) | The layers the caller may install this entry at now: the ones they may write to and the org policy allows. | 
**name** | **String** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


