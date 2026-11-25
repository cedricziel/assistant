# assistant_api.model.CreateIntegrationRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**authKind** | [**IntegrationAuthKind**](IntegrationAuthKind.md) | Required for a custom URL and for a catalog entry whose auth kind is unknown (choose `none`, `header` or `oauth`). A `mixed` catalog entry may be narrowed to `header` or `oauth`. | [optional] 
**catalogEntryId** | **String** | Install this catalog entry. Its URL, auth kind and header template are copied at install time. | [optional] 
**connectUrl** | **String** | Install a custom remote MCP server. Needs `name` and `auth_kind`. | [optional] 
**credentialMode** | [**CredentialMode**](CredentialMode.md) | Defaults to `per_user` for OAuth and `shared` otherwise. User-layer integrations are always `per_user`. | [optional] 
**enabled** | **bool** | Defaults to `true`. | [optional] 
**headerName** | **String** | Header that carries the credential. Required for `header` auth on a custom URL or an unknown catalog entry; rejected when the catalog entry supplies its own template. | [optional] 
**headerValuePrefix** | **String** | Text before the secret in the header value. Same rules as `header_name`. | [optional] 
**layer** | [**IntegrationLayer**](IntegrationLayer.md) |  | 
**name** | **String** | Defaults to the catalog entry's name. Required for a custom URL. | [optional] 
**slug** | **String** | Kebab-case, 1–32 characters. Defaults to the catalog entry's slug. | [optional] 
**spaceId** | **String** | Required for the space layer, rejected otherwise. | [optional] 
**trust** | [**IntegrationTrust**](IntegrationTrust.md) | Defaults to `confirm`. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


