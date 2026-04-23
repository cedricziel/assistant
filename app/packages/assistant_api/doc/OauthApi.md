# assistant_api.api.OauthApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**authorizeGet**](OauthApi.md#authorizeget) | **GET** /oauth/authorize | GET /oauth/authorize — render login form.
[**authorizePost**](OauthApi.md#authorizepost) | **POST** /oauth/authorize | POST /oauth/authorize — validate credentials, generate auth code, redirect.
[**deviceInitiate**](OauthApi.md#deviceinitiate) | **POST** /oauth/device | POST /oauth/device — initiate the device authorization flow.
[**deviceVerifyPage**](OauthApi.md#deviceverifypage) | **GET** /oauth/device/verify | GET /oauth/device/verify — render page where user enters the code.
[**deviceVerifySubmit**](OauthApi.md#deviceverifysubmit) | **POST** /oauth/device/verify | POST /oauth/device/verify — user approves the device code.
[**metadata**](OauthApi.md#metadata) | **GET** /.well-known/oauth-authorization-server | GET /.well-known/oauth-authorization-server
[**register**](OauthApi.md#register) | **POST** /oauth/register | POST /oauth/register — register a new OAuth2 client.
[**revoke**](OauthApi.md#revoke) | **POST** /oauth/revoke | POST /oauth/revoke — revoke a refresh token.
[**token**](OauthApi.md#token) | **POST** /oauth/token | POST /oauth/token


# **authorizeGet**
> authorizeGet(responseType, clientId, redirectUri, state, codeChallenge, codeChallengeMethod, scope)

GET /oauth/authorize — render login form.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOauthApi();
final String responseType = responseType_example; // String | 
final String clientId = clientId_example; // String | 
final String redirectUri = redirectUri_example; // String | 
final String state = state_example; // String | 
final String codeChallenge = codeChallenge_example; // String | 
final String codeChallengeMethod = codeChallengeMethod_example; // String | 
final String scope = scope_example; // String | 

try {
    api.authorizeGet(responseType, clientId, redirectUri, state, codeChallenge, codeChallengeMethod, scope);
} on DioException catch (e) {
    print('Exception when calling OauthApi->authorizeGet: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **responseType** | **String**|  | [optional] 
 **clientId** | **String**|  | [optional] 
 **redirectUri** | **String**|  | [optional] 
 **state** | **String**|  | [optional] 
 **codeChallenge** | **String**|  | [optional] 
 **codeChallengeMethod** | **String**|  | [optional] 
 **scope** | **String**|  | [optional] 

### Return type

void (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **authorizePost**
> authorizePost(clientId, redirectUri, codeChallenge, email, password, scope, state)

POST /oauth/authorize — validate credentials, generate auth code, redirect.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOauthApi();
final String clientId = clientId_example; // String | 
final String redirectUri = redirectUri_example; // String | 
final String codeChallenge = codeChallenge_example; // String | 
final String email = email_example; // String | 
final String password = password_example; // String | 
final String scope = scope_example; // String | 
final String state = state_example; // String | 

try {
    api.authorizePost(clientId, redirectUri, codeChallenge, email, password, scope, state);
} on DioException catch (e) {
    print('Exception when calling OauthApi->authorizePost: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **clientId** | **String**|  | 
 **redirectUri** | **String**|  | 
 **codeChallenge** | **String**|  | [optional] 
 **email** | **String**|  | [optional] 
 **password** | **String**|  | [optional] 
 **scope** | **String**|  | [optional] 
 **state** | **String**|  | [optional] 

### Return type

void (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/x-www-form-urlencoded
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deviceInitiate**
> DeviceCodeResponseSchema deviceInitiate(clientId, scope)

POST /oauth/device — initiate the device authorization flow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOauthApi();
final String clientId = clientId_example; // String | 
final String scope = scope_example; // String | 

try {
    final response = api.deviceInitiate(clientId, scope);
    print(response);
} on DioException catch (e) {
    print('Exception when calling OauthApi->deviceInitiate: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **clientId** | **String**|  | 
 **scope** | **String**|  | [optional] 

### Return type

[**DeviceCodeResponseSchema**](DeviceCodeResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/x-www-form-urlencoded
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deviceVerifyPage**
> deviceVerifyPage(userCode)

GET /oauth/device/verify — render page where user enters the code.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOauthApi();
final String userCode = userCode_example; // String | 

try {
    api.deviceVerifyPage(userCode);
} on DioException catch (e) {
    print('Exception when calling OauthApi->deviceVerifyPage: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **userCode** | **String**|  | [optional] 

### Return type

void (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deviceVerifySubmit**
> deviceVerifySubmit(userCode, userId)

POST /oauth/device/verify — user approves the device code.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOauthApi();
final String userCode = userCode_example; // String | 
final String userId = userId_example; // String | In a real deployment, user_id comes from the authenticated session. For now, accept it as a form field or default to \\\"anonymous\\\".

try {
    api.deviceVerifySubmit(userCode, userId);
} on DioException catch (e) {
    print('Exception when calling OauthApi->deviceVerifySubmit: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **userCode** | **String**|  | 
 **userId** | **String**| In a real deployment, user_id comes from the authenticated session. For now, accept it as a form field or default to \\\"anonymous\\\". | [optional] 

### Return type

void (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/x-www-form-urlencoded
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **metadata**
> ServerMetadata metadata()

GET /.well-known/oauth-authorization-server

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOauthApi();

try {
    final response = api.metadata();
    print(response);
} on DioException catch (e) {
    print('Exception when calling OauthApi->metadata: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**ServerMetadata**](ServerMetadata.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **register**
> ClientInfoSchema register(clientRegistrationSchema)

POST /oauth/register — register a new OAuth2 client.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOauthApi();
final ClientRegistrationSchema clientRegistrationSchema = ; // ClientRegistrationSchema | 

try {
    final response = api.register(clientRegistrationSchema);
    print(response);
} on DioException catch (e) {
    print('Exception when calling OauthApi->register: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **clientRegistrationSchema** | [**ClientRegistrationSchema**](ClientRegistrationSchema.md)|  | 

### Return type

[**ClientInfoSchema**](ClientInfoSchema.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **revoke**
> revoke(token, clientId, tokenTypeHint)

POST /oauth/revoke — revoke a refresh token.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOauthApi();
final String token = token_example; // String | 
final String clientId = clientId_example; // String | 
final String tokenTypeHint = tokenTypeHint_example; // String | 

try {
    api.revoke(token, clientId, tokenTypeHint);
} on DioException catch (e) {
    print('Exception when calling OauthApi->revoke: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **token** | **String**|  | 
 **clientId** | **String**|  | [optional] 
 **tokenTypeHint** | **String**|  | [optional] 

### Return type

void (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/x-www-form-urlencoded
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **token**
> TokenResponse token(grantType, clientId, code, codeVerifier, deviceCode, redirectUri, refreshToken)

POST /oauth/token

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOauthApi();
final String grantType = grantType_example; // String | 
final String clientId = clientId_example; // String | 
final String code = code_example; // String | Authorization code (for `authorization_code` grant).
final String codeVerifier = codeVerifier_example; // String | PKCE verifier (RFC 7636).
final String deviceCode = deviceCode_example; // String | Device code (for device code grant).
final String redirectUri = redirectUri_example; // String | 
final String refreshToken = refreshToken_example; // String | Refresh token (for `refresh_token` grant).

try {
    final response = api.token(grantType, clientId, code, codeVerifier, deviceCode, redirectUri, refreshToken);
    print(response);
} on DioException catch (e) {
    print('Exception when calling OauthApi->token: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **grantType** | **String**|  | 
 **clientId** | **String**|  | [optional] 
 **code** | **String**| Authorization code (for `authorization_code` grant). | [optional] 
 **codeVerifier** | **String**| PKCE verifier (RFC 7636). | [optional] 
 **deviceCode** | **String**| Device code (for device code grant). | [optional] 
 **redirectUri** | **String**|  | [optional] 
 **refreshToken** | **String**| Refresh token (for `refresh_token` grant). | [optional] 

### Return type

[**TokenResponse**](TokenResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/x-www-form-urlencoded
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

