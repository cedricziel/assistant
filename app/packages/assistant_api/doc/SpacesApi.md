# assistant_api.api.SpacesApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createSpace**](SpacesApi.md#createspace) | **POST** /api/orgs/{org_id}/spaces | &#x60;POST /api/orgs/{org_id}/spaces&#x60; — create a space (org-admin only).
[**deleteSpace**](SpacesApi.md#deletespace) | **DELETE** /api/orgs/{org_id}/spaces/{id} | &#x60;DELETE /api/orgs/{org_id}/spaces/{id}&#x60; — delete a space (org-admin only).
[**getSpace**](SpacesApi.md#getspace) | **GET** /api/orgs/{org_id}/spaces/{id} | &#x60;GET /api/orgs/{org_id}/spaces/{id}&#x60; — get space detail.
[**listSpaces**](SpacesApi.md#listspaces) | **GET** /api/orgs/{org_id}/spaces | &#x60;GET /api/orgs/{org_id}/spaces&#x60; — list spaces filtered by membership.
[**updateSpace**](SpacesApi.md#updatespace) | **PATCH** /api/orgs/{org_id}/spaces/{id} | &#x60;PATCH /api/orgs/{org_id}/spaces/{id}&#x60; — update space name.


# **createSpace**
> SpaceDetail createSpace(orgId, createSpaceRequest)

`POST /api/orgs/{org_id}/spaces` — create a space (org-admin only).

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getSpacesApi();
final String orgId = orgId_example; // String | Organization ID
final CreateSpaceRequest createSpaceRequest = ; // CreateSpaceRequest | 

try {
    final response = api.createSpace(orgId, createSpaceRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling SpacesApi->createSpace: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **createSpaceRequest** | [**CreateSpaceRequest**](CreateSpaceRequest.md)|  | 

### Return type

[**SpaceDetail**](SpaceDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteSpace**
> deleteSpace(orgId, id)

`DELETE /api/orgs/{org_id}/spaces/{id}` — delete a space (org-admin only).

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getSpacesApi();
final String orgId = orgId_example; // String | Organization ID
final String id = id_example; // String | Space ID

try {
    api.deleteSpace(orgId, id);
} on DioException catch (e) {
    print('Exception when calling SpacesApi->deleteSpace: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **id** | **String**| Space ID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getSpace**
> SpaceDetail getSpace(orgId, id)

`GET /api/orgs/{org_id}/spaces/{id}` — get space detail.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getSpacesApi();
final String orgId = orgId_example; // String | Organization ID
final String id = id_example; // String | Space ID

try {
    final response = api.getSpace(orgId, id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling SpacesApi->getSpace: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **id** | **String**| Space ID | 

### Return type

[**SpaceDetail**](SpaceDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listSpaces**
> BuiltList<SpaceSummary> listSpaces(orgId)

`GET /api/orgs/{org_id}/spaces` — list spaces filtered by membership.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getSpacesApi();
final String orgId = orgId_example; // String | Organization ID

try {
    final response = api.listSpaces(orgId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling SpacesApi->listSpaces: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 

### Return type

[**BuiltList&lt;SpaceSummary&gt;**](SpaceSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateSpace**
> SpaceDetail updateSpace(orgId, id, updateSpaceRequest)

`PATCH /api/orgs/{org_id}/spaces/{id}` — update space name.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getSpacesApi();
final String orgId = orgId_example; // String | Organization ID
final String id = id_example; // String | Space ID
final UpdateSpaceRequest updateSpaceRequest = ; // UpdateSpaceRequest | 

try {
    final response = api.updateSpace(orgId, id, updateSpaceRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling SpacesApi->updateSpace: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **id** | **String**| Space ID | 
 **updateSpaceRequest** | [**UpdateSpaceRequest**](UpdateSpaceRequest.md)|  | 

### Return type

[**SpaceDetail**](SpaceDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

