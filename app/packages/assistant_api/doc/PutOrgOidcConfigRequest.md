# assistant_api.model.PutOrgOidcConfigRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**allowedEmailDomains** | **BuiltList&lt;String&gt;** |  | [optional] 
**autoProvision** | **bool** | Defaults to `true`. | [optional] 
**clientId** | **String** |  | 
**clientSecret** | **String** | Omit to keep the stored secret, `null` to clear it. | [optional] 
**issuerUrl** | **String** | `https` URL, or `http` for `localhost` / loopback addresses. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


