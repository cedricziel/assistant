# assistant_api.api.BindingsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createBinding**](BindingsApi.md#createbinding) | **POST** /api/orgs/{org_id}/spaces/{space_id}/bindings | &#x60;POST /api/orgs/{org_id}/spaces/{space_id}/bindings&#x60; — bind a persona to an interface.
[**deleteBinding**](BindingsApi.md#deletebinding) | **DELETE** /api/orgs/{org_id}/spaces/{space_id}/bindings/{id} | &#x60;DELETE /api/orgs/{org_id}/spaces/{space_id}/bindings/{id}&#x60; — delete a binding.
[**listBindings**](BindingsApi.md#listbindings) | **GET** /api/orgs/{org_id}/spaces/{space_id}/bindings | &#x60;GET /api/orgs/{org_id}/spaces/{space_id}/bindings&#x60; — list bindings.


# **createBinding**
> BindingResponse createBinding(orgId, spaceId, createBindingRequest)

`POST /api/orgs/{org_id}/spaces/{space_id}/bindings` — bind a persona to an interface.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getBindingsApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID
final CreateBindingRequest createBindingRequest = ; // CreateBindingRequest | 

try {
    final response = api.createBinding(orgId, spaceId, createBindingRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling BindingsApi->createBinding: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 
 **createBindingRequest** | [**CreateBindingRequest**](CreateBindingRequest.md)|  | 

### Return type

[**BindingResponse**](BindingResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteBinding**
> deleteBinding(orgId, spaceId, id)

`DELETE /api/orgs/{org_id}/spaces/{space_id}/bindings/{id}` — delete a binding.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getBindingsApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID
final String id = id_example; // String | Binding ID

try {
    api.deleteBinding(orgId, spaceId, id);
} on DioException catch (e) {
    print('Exception when calling BindingsApi->deleteBinding: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 
 **id** | **String**| Binding ID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listBindings**
> BuiltList<BindingResponse> listBindings(orgId, spaceId)

`GET /api/orgs/{org_id}/spaces/{space_id}/bindings` — list bindings.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getBindingsApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID

try {
    final response = api.listBindings(orgId, spaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling BindingsApi->listBindings: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 

### Return type

[**BuiltList&lt;BindingResponse&gt;**](BindingResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

