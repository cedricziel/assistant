# assistant_api.model.OAuthFlows

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**authorizationCode** | [**AuthorizationCodeOAuthFlow**](AuthorizationCodeOAuthFlow.md) | Configuration for the OAuth Authorization Code flow. | [optional] 
**clientCredentials** | [**ClientCredentialsOAuthFlow**](ClientCredentialsOAuthFlow.md) | Configuration for the OAuth Client Credentials flow. | [optional] 
**deviceCode** | [**DeviceCodeOAuthFlow**](DeviceCodeOAuthFlow.md) | Configuration for the OAuth Device Code flow. | [optional] 
**implicit** | [**ImplicitOAuthFlow**](ImplicitOAuthFlow.md) | Deprecated: Use Authorization Code + PKCE instead. | [optional] 
**password** | [**PasswordOAuthFlow**](PasswordOAuthFlow.md) | Deprecated: Use Authorization Code + PKCE or Device Code. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


