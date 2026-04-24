# assistant_api.api.UsersApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createUser**](UsersApi.md#createuser) | **POST** /api/orgs/{org_id}/users | &#x60;POST /api/orgs/{org_id}/users&#x60; — create/invite a user.
[**deleteUser**](UsersApi.md#deleteuser) | **DELETE** /api/orgs/{org_id}/users/{id} | &#x60;DELETE /api/orgs/{org_id}/users/{id}&#x60; — remove a user (org-admin only).
[**getUser**](UsersApi.md#getuser) | **GET** /api/orgs/{org_id}/users/{id} | &#x60;GET /api/orgs/{org_id}/users/{id}&#x60; — get user detail.
[**listUsers**](UsersApi.md#listusers) | **GET** /api/orgs/{org_id}/users | &#x60;GET /api/orgs/{org_id}/users&#x60; — list users in an org.
[**updateUser**](UsersApi.md#updateuser) | **PATCH** /api/orgs/{org_id}/users/{id} | &#x60;PATCH /api/orgs/{org_id}/users/{id}&#x60; — update user name or email.


# **createUser**
> UserDetail createUser(orgId, createUserRequest)

`POST /api/orgs/{org_id}/users` — create/invite a user.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getUsersApi();
final String orgId = orgId_example; // String | Organization ID
final CreateUserRequest createUserRequest = ; // CreateUserRequest | 

try {
    final response = api.createUser(orgId, createUserRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling UsersApi->createUser: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **createUserRequest** | [**CreateUserRequest**](CreateUserRequest.md)|  | 

### Return type

[**UserDetail**](UserDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteUser**
> deleteUser(orgId, id)

`DELETE /api/orgs/{org_id}/users/{id}` — remove a user (org-admin only).

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getUsersApi();
final String orgId = orgId_example; // String | Organization ID
final String id = id_example; // String | User ID

try {
    api.deleteUser(orgId, id);
} on DioException catch (e) {
    print('Exception when calling UsersApi->deleteUser: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **id** | **String**| User ID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getUser**
> UserDetail getUser(orgId, id)

`GET /api/orgs/{org_id}/users/{id}` — get user detail.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getUsersApi();
final String orgId = orgId_example; // String | Organization ID
final String id = id_example; // String | User ID

try {
    final response = api.getUser(orgId, id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling UsersApi->getUser: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **id** | **String**| User ID | 

### Return type

[**UserDetail**](UserDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listUsers**
> BuiltList<UserSummary> listUsers(orgId)

`GET /api/orgs/{org_id}/users` — list users in an org.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getUsersApi();
final String orgId = orgId_example; // String | Organization ID

try {
    final response = api.listUsers(orgId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling UsersApi->listUsers: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 

### Return type

[**BuiltList&lt;UserSummary&gt;**](UserSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateUser**
> UserDetail updateUser(orgId, id, updateUserRequest)

`PATCH /api/orgs/{org_id}/users/{id}` — update user name or email.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getUsersApi();
final String orgId = orgId_example; // String | Organization ID
final String id = id_example; // String | User ID
final UpdateUserRequest updateUserRequest = ; // UpdateUserRequest | 

try {
    final response = api.updateUser(orgId, id, updateUserRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling UsersApi->updateUser: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **id** | **String**| User ID | 
 **updateUserRequest** | [**UpdateUserRequest**](UpdateUserRequest.md)|  | 

### Return type

[**UserDetail**](UserDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

