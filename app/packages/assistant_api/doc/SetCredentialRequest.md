# assistant_api.model.SetCredentialRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**headerName** | **String** | The header that carries the value. Only accepted when the integration has no header name yet, and then only from someone who manages its layer. | [optional] 
**value** | **String** | The secret that fills the header template, e.g. the token after `Bearer `. Surrounding whitespace is dropped. It is stored encrypted and never returned. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


