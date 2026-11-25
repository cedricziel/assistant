# assistant_api.api.WebhooksApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createWebhook**](WebhooksApi.md#createwebhook) | **POST** /api/webhooks | &#x60;POST /api/webhooks&#x60; — create a new webhook.
[**deleteWebhook**](WebhooksApi.md#deletewebhook) | **DELETE** /api/webhooks/{id} | &#x60;DELETE /api/webhooks/{id}&#x60; — delete a webhook.
[**getWebhook**](WebhooksApi.md#getwebhook) | **GET** /api/webhooks/{id} | &#x60;GET /api/webhooks/{id}&#x60; — get a webhook by ID.
[**listWebhooks**](WebhooksApi.md#listwebhooks) | **GET** /api/webhooks | &#x60;GET /api/webhooks&#x60; — list all webhooks.
[**rotateSecret**](WebhooksApi.md#rotatesecret) | **POST** /api/webhooks/{id}/rotate-secret | &#x60;POST /api/webhooks/{id}/rotate-secret&#x60; — regenerate a webhook&#39;s signing secret.
[**toggleWebhook**](WebhooksApi.md#togglewebhook) | **POST** /api/webhooks/{id}/toggle | &#x60;POST /api/webhooks/{id}/toggle&#x60; — toggle a webhook&#39;s active state.
[**updateWebhook**](WebhooksApi.md#updatewebhook) | **PATCH** /api/webhooks/{id} | &#x60;PATCH /api/webhooks/{id}&#x60; — update a webhook.
[**verifyWebhook**](WebhooksApi.md#verifywebhook) | **POST** /api/webhooks/{id}/verify | &#x60;POST /api/webhooks/{id}/verify&#x60; — send a signed test payload to the webhook URL.


# **createWebhook**
> WebhookResponse createWebhook(createWebhookRequest, xSpaceId)

`POST /api/webhooks` — create a new webhook.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final CreateWebhookRequest createWebhookRequest = ; // CreateWebhookRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.createWebhook(createWebhookRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->createWebhook: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createWebhookRequest** | [**CreateWebhookRequest**](CreateWebhookRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**WebhookResponse**](WebhookResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteWebhook**
> deleteWebhook(id, xSpaceId)

`DELETE /api/webhooks/{id}` — delete a webhook.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String id = id_example; // String | Webhook ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.deleteWebhook(id, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->deleteWebhook: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Webhook ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getWebhook**
> WebhookResponse getWebhook(id, xSpaceId)

`GET /api/webhooks/{id}` — get a webhook by ID.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String id = id_example; // String | Webhook ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.getWebhook(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->getWebhook: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Webhook ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**WebhookResponse**](WebhookResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listWebhooks**
> BuiltList<WebhookResponse> listWebhooks(xSpaceId)

`GET /api/webhooks` — list all webhooks.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.listWebhooks(xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->listWebhooks: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**BuiltList&lt;WebhookResponse&gt;**](WebhookResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **rotateSecret**
> RotateSecretResponse rotateSecret(id, xSpaceId)

`POST /api/webhooks/{id}/rotate-secret` — regenerate a webhook's signing secret.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String id = id_example; // String | Webhook ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.rotateSecret(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->rotateSecret: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Webhook ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**RotateSecretResponse**](RotateSecretResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **toggleWebhook**
> WebhookResponse toggleWebhook(id, xSpaceId)

`POST /api/webhooks/{id}/toggle` — toggle a webhook's active state.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String id = id_example; // String | Webhook ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.toggleWebhook(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->toggleWebhook: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Webhook ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**WebhookResponse**](WebhookResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateWebhook**
> WebhookResponse updateWebhook(id, updateWebhookRequest, xSpaceId)

`PATCH /api/webhooks/{id}` — update a webhook.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String id = id_example; // String | Webhook ID
final UpdateWebhookRequest updateWebhookRequest = ; // UpdateWebhookRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.updateWebhook(id, updateWebhookRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->updateWebhook: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Webhook ID | 
 **updateWebhookRequest** | [**UpdateWebhookRequest**](UpdateWebhookRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**WebhookResponse**](WebhookResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **verifyWebhook**
> VerifyWebhookResponse verifyWebhook(id, xSpaceId)

`POST /api/webhooks/{id}/verify` — send a signed test payload to the webhook URL.

Always returns HTTP 200; the `success` field in the body indicates whether the remote endpoint responded with a 2xx status.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String id = id_example; // String | Webhook ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.verifyWebhook(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->verifyWebhook: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Webhook ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**VerifyWebhookResponse**](VerifyWebhookResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

