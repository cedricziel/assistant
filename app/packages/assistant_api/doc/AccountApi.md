# assistant_api.api.AccountApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**changePassword**](AccountApi.md#changepassword) | **POST** /api/users/me/password | &#x60;POST /api/users/me/password&#x60; — change the caller&#39;s password and revoke all their refresh tokens. The current access JWT keeps working until it naturally expires; API keys are untouched.
[**getCurrentUser**](AccountApi.md#getcurrentuser) | **GET** /api/users/me | &#x60;GET /api/users/me&#x60; — return the caller&#39;s &#x60;UserDetail&#x60;. Works in any &#x60;auth_mode&#x60;.
[**updateCurrentUser**](AccountApi.md#updatecurrentuser) | **PATCH** /api/users/me | &#x60;PATCH /api/users/me&#x60; — update the caller&#39;s name and/or email.


# **changePassword**
> changePassword(changePasswordRequest)

`POST /api/users/me/password` — change the caller's password and revoke all their refresh tokens. The current access JWT keeps working until it naturally expires; API keys are untouched.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getAccountApi();
final ChangePasswordRequest changePasswordRequest = ; // ChangePasswordRequest | 

try {
    api.changePassword(changePasswordRequest);
} on DioException catch (e) {
    print('Exception when calling AccountApi->changePassword: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **changePasswordRequest** | [**ChangePasswordRequest**](ChangePasswordRequest.md)|  | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getCurrentUser**
> UserDetail getCurrentUser()

`GET /api/users/me` — return the caller's `UserDetail`. Works in any `auth_mode`.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getAccountApi();

try {
    final response = api.getCurrentUser();
    print(response);
} on DioException catch (e) {
    print('Exception when calling AccountApi->getCurrentUser: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**UserDetail**](UserDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateCurrentUser**
> UpdateCurrentUserResponse updateCurrentUser(updateCurrentUserRequest)

`PATCH /api/users/me` — update the caller's name and/or email.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getAccountApi();
final UpdateCurrentUserRequest updateCurrentUserRequest = ; // UpdateCurrentUserRequest | 

try {
    final response = api.updateCurrentUser(updateCurrentUserRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling AccountApi->updateCurrentUser: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **updateCurrentUserRequest** | [**UpdateCurrentUserRequest**](UpdateCurrentUserRequest.md)|  | 

### Return type

[**UpdateCurrentUserResponse**](UpdateCurrentUserResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

