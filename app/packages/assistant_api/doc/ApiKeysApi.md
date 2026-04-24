# assistant_api.api.ApiKeysApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createApiKey**](ApiKeysApi.md#createapikey) | **POST** /api/users/me/api-keys | &#x60;POST /api/users/me/api-keys&#x60; — create a new scoped API key.
[**deleteApiKey**](ApiKeysApi.md#deleteapikey) | **DELETE** /api/users/me/api-keys/{id} | &#x60;DELETE /api/users/me/api-keys/{id}&#x60; — revoke an API key.
[**listApiKeys**](ApiKeysApi.md#listapikeys) | **GET** /api/users/me/api-keys | &#x60;GET /api/users/me/api-keys&#x60; — list the caller&#39;s API keys.


# **createApiKey**
> CreateApiKeyResponse createApiKey(createApiKeyRequest)

`POST /api/users/me/api-keys` — create a new scoped API key.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getApiKeysApi();
final CreateApiKeyRequest createApiKeyRequest = ; // CreateApiKeyRequest | 

try {
    final response = api.createApiKey(createApiKeyRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ApiKeysApi->createApiKey: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createApiKeyRequest** | [**CreateApiKeyRequest**](CreateApiKeyRequest.md)|  | 

### Return type

[**CreateApiKeyResponse**](CreateApiKeyResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteApiKey**
> deleteApiKey(id)

`DELETE /api/users/me/api-keys/{id}` — revoke an API key.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getApiKeysApi();
final String id = id_example; // String | API key ID

try {
    api.deleteApiKey(id);
} on DioException catch (e) {
    print('Exception when calling ApiKeysApi->deleteApiKey: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| API key ID | 

### Return type

void (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listApiKeys**
> BuiltList<ApiKeySummary> listApiKeys()

`GET /api/users/me/api-keys` — list the caller's API keys.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getApiKeysApi();

try {
    final response = api.listApiKeys();
    print(response);
} on DioException catch (e) {
    print('Exception when calling ApiKeysApi->listApiKeys: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**BuiltList&lt;ApiKeySummary&gt;**](ApiKeySummary.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

