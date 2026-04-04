# assistant_api.api.PersonasApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**listPersonas**](PersonasApi.md#listpersonas) | **GET** /api/personas | &#x60;GET /api/personas&#x60; — list all personas defined on the server.
[**setActivePersona**](PersonasApi.md#setactivepersona) | **POST** /api/personas/active | &#x60;POST /api/personas/active&#x60; — switch the active persona for the session.


# **listPersonas**
> BuiltList<PersonaSummary> listPersonas()

`GET /api/personas` — list all personas defined on the server.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();

try {
    final response = api.listPersonas();
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->listPersonas: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**BuiltList&lt;PersonaSummary&gt;**](PersonaSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **setActivePersona**
> PersonaSummary setActivePersona(setActivePersonaRequest)

`POST /api/personas/active` — switch the active persona for the session.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final SetActivePersonaRequest setActivePersonaRequest = ; // SetActivePersonaRequest | 

try {
    final response = api.setActivePersona(setActivePersonaRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->setActivePersona: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **setActivePersonaRequest** | [**SetActivePersonaRequest**](SetActivePersonaRequest.md)|  | 

### Return type

[**PersonaSummary**](PersonaSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

