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
> WebhookResponse createWebhook(createWebhookRequest)

`POST /api/webhooks` — create a new webhook.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final CreateWebhookRequest createWebhookRequest = ; // CreateWebhookRequest | 

try {
    final response = api.createWebhook(createWebhookRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->createWebhook: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createWebhookRequest** | [**CreateWebhookRequest**](CreateWebhookRequest.md)|  | 

### Return type

[**WebhookResponse**](WebhookResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteWebhook**
> deleteWebhook(id)

`DELETE /api/webhooks/{id}` — delete a webhook.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String id = id_example; // String | Webhook ID

try {
    api.deleteWebhook(id);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->deleteWebhook: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Webhook ID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getWebhook**
> WebhookResponse getWebhook(id)

`GET /api/webhooks/{id}` — get a webhook by ID.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String id = id_example; // String | Webhook ID

try {
    final response = api.getWebhook(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->getWebhook: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Webhook ID | 

### Return type

[**WebhookResponse**](WebhookResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listWebhooks**
> BuiltList<WebhookResponse> listWebhooks()

`GET /api/webhooks` — list all webhooks.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();

try {
    final response = api.listWebhooks();
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->listWebhooks: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**BuiltList&lt;WebhookResponse&gt;**](WebhookResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **rotateSecret**
> RotateSecretResponse rotateSecret(id)

`POST /api/webhooks/{id}/rotate-secret` — regenerate a webhook's signing secret.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String id = id_example; // String | Webhook ID

try {
    final response = api.rotateSecret(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->rotateSecret: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Webhook ID | 

### Return type

[**RotateSecretResponse**](RotateSecretResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **toggleWebhook**
> WebhookResponse toggleWebhook(id)

`POST /api/webhooks/{id}/toggle` — toggle a webhook's active state.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String id = id_example; // String | Webhook ID

try {
    final response = api.toggleWebhook(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->toggleWebhook: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Webhook ID | 

### Return type

[**WebhookResponse**](WebhookResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateWebhook**
> WebhookResponse updateWebhook(id, updateWebhookRequest)

`PATCH /api/webhooks/{id}` — update a webhook.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String id = id_example; // String | Webhook ID
final UpdateWebhookRequest updateWebhookRequest = ; // UpdateWebhookRequest | 

try {
    final response = api.updateWebhook(id, updateWebhookRequest);
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

### Return type

[**WebhookResponse**](WebhookResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **verifyWebhook**
> VerifyWebhookResponse verifyWebhook(id)

`POST /api/webhooks/{id}/verify` — send a signed test payload to the webhook URL.

Always returns HTTP 200; the `success` field in the body indicates whether the remote endpoint responded with a 2xx status.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebhooksApi();
final String id = id_example; // String | Webhook ID

try {
    final response = api.verifyWebhook(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebhooksApi->verifyWebhook: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Webhook ID | 

### Return type

[**VerifyWebhookResponse**](VerifyWebhookResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

