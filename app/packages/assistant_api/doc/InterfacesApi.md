# assistant_api.api.InterfacesApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createInterface**](InterfacesApi.md#createinterface) | **POST** /api/orgs/{org_id}/spaces/{space_id}/interfaces | &#x60;POST /api/orgs/{org_id}/spaces/{space_id}/interfaces&#x60; — create an interface instance.
[**deleteInterface**](InterfacesApi.md#deleteinterface) | **DELETE** /api/orgs/{org_id}/spaces/{space_id}/interfaces/{id} | &#x60;DELETE /api/orgs/{org_id}/spaces/{space_id}/interfaces/{id}&#x60; — delete an interface instance.
[**listInterfaces**](InterfacesApi.md#listinterfaces) | **GET** /api/orgs/{org_id}/spaces/{space_id}/interfaces | &#x60;GET /api/orgs/{org_id}/spaces/{space_id}/interfaces&#x60; — list interface instances.


# **createInterface**
> InterfaceInstanceResponse createInterface(orgId, spaceId, createInterfaceInstanceRequest)

`POST /api/orgs/{org_id}/spaces/{space_id}/interfaces` — create an interface instance.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getInterfacesApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID
final CreateInterfaceInstanceRequest createInterfaceInstanceRequest = ; // CreateInterfaceInstanceRequest | 

try {
    final response = api.createInterface(orgId, spaceId, createInterfaceInstanceRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling InterfacesApi->createInterface: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 
 **createInterfaceInstanceRequest** | [**CreateInterfaceInstanceRequest**](CreateInterfaceInstanceRequest.md)|  | 

### Return type

[**InterfaceInstanceResponse**](InterfaceInstanceResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteInterface**
> deleteInterface(orgId, spaceId, id)

`DELETE /api/orgs/{org_id}/spaces/{space_id}/interfaces/{id}` — delete an interface instance.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getInterfacesApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID
final String id = id_example; // String | Interface instance ID

try {
    api.deleteInterface(orgId, spaceId, id);
} on DioException catch (e) {
    print('Exception when calling InterfacesApi->deleteInterface: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 
 **id** | **String**| Interface instance ID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listInterfaces**
> BuiltList<InterfaceInstanceResponse> listInterfaces(orgId, spaceId)

`GET /api/orgs/{org_id}/spaces/{space_id}/interfaces` — list interface instances.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getInterfacesApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID

try {
    final response = api.listInterfaces(orgId, spaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling InterfacesApi->listInterfaces: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 

### Return type

[**BuiltList&lt;InterfaceInstanceResponse&gt;**](InterfaceInstanceResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

