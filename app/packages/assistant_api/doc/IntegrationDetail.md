# assistant_api.model.IntegrationDetail

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**authKind** | [**IntegrationAuthKind**](IntegrationAuthKind.md) |  | 
**catalogEntryId** | **String** |  | [optional] 
**connectUrl** | **String** |  | 
**createdAt** | [**DateTime**](DateTime.md) |  | 
**credentialMode** | [**CredentialMode**](CredentialMode.md) |  | 
**enabled** | **bool** |  | 
**hasCredential** | **bool** |  | 
**headerName** | **String** | Header that carries the credential, for `header` and `mixed` auth. | [optional] 
**headerValuePrefix** | **String** | Text before the secret in the header value, e.g. `Bearer `. | [optional] 
**id** | **String** | `int_<uuid>`. | 
**lastConnectedAt** | [**DateTime**](DateTime.md) |  | [optional] 
**layer** | [**IntegrationLayer**](IntegrationLayer.md) |  | 
**name** | **String** |  | 
**ownerUserId** | **String** |  | [optional] 
**slug** | **String** |  | 
**spaceAccess** | [**BuiltList&lt;SpaceAccessEntry&gt;**](SpaceAccessEntry.md) | Space rows of an org-layer integration, limited to the spaces the caller belongs to unless the caller is an org admin. | 
**spaceId** | **String** |  | [optional] 
**status** | [**IntegrationStatus**](IntegrationStatus.md) |  | 
**toolCount** | **int** |  | [optional] 
**trust** | [**IntegrationTrust**](IntegrationTrust.md) |  | 
**updatedAt** | [**DateTime**](DateTime.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


