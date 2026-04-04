# assistant_api.model.AgentCardSignature

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**header** | [**BuiltMap&lt;String, JsonObject&gt;**](JsonObject.md) | The unprotected JWS header values. | [optional] 
**protected** | **String** | The protected JWS header, base64url-encoded JSON object. | 
**signature** | **String** | The computed signature, base64url-encoded. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


