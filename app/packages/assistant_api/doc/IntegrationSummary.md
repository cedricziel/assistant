# assistant_api.model.IntegrationSummary

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**authKind** | [**IntegrationAuthKind**](IntegrationAuthKind.md) |  | 
**catalogEntryId** | **String** | The catalog entry it was installed from; absent for a custom URL. | [optional] 
**connectUrl** | **String** |  | 
**createdAt** | [**DateTime**](DateTime.md) |  | 
**credentialMode** | [**CredentialMode**](CredentialMode.md) |  | 
**enabled** | **bool** |  | 
**hasCredential** | **bool** | Whether a credential is stored for the caller's subject: the shared credential, or the caller's own for a per-user integration. The value itself is never returned. | 
**id** | **String** | `int_<uuid>`. | 
**lastConnectedAt** | [**DateTime**](DateTime.md) | The last time a connection for the caller's subject succeeded. | [optional] 
**layer** | [**IntegrationLayer**](IntegrationLayer.md) |  | 
**name** | **String** |  | 
**ownerUserId** | **String** | The owner of a user-layer integration. | [optional] 
**slug** | **String** | Unique within its layer and owner. Part of its tool names. | 
**spaceId** | **String** | The space of a space-layer integration. | [optional] 
**status** | [**IntegrationStatus**](IntegrationStatus.md) | Whether the caller can use it. For an org admin looking at another user's integration, the status is the owner's. | 
**toolCount** | **int** | The number of tools that connection found. | [optional] 
**trust** | [**IntegrationTrust**](IntegrationTrust.md) |  | 
**updatedAt** | [**DateTime**](DateTime.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


