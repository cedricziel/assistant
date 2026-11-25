# assistant_api.model.IntegrationPolicyResponse

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**allowCustomUrls** | **bool** |  | 
**allowUserIntegrations** | **bool** |  | 
**allowedCatalogIds** | **BuiltList&lt;String&gt;** | Catalog entry ids allowed when `catalog_mode` is `allowlist`. | 
**catalogMode** | [**CatalogMode**](CatalogMode.md) |  | 
**updatedAt** | [**DateTime**](DateTime.md) | Absent until an org admin first saves the policy. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


