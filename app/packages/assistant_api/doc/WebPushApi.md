# assistant_api.api.WebPushApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**subscribe**](WebPushApi.md#subscribe) | **POST** /api/push/subscribe | &#x60;POST /api/push/subscribe&#x60;
[**unsubscribe**](WebPushApi.md#unsubscribe) | **DELETE** /api/push/subscribe | &#x60;DELETE /api/push/subscribe&#x60;
[**vapidPublicKey**](WebPushApi.md#vapidpublickey) | **GET** /api/push/vapid-public-key | &#x60;GET /api/push/vapid-public-key&#x60;


# **subscribe**
> subscribe(subscribeRequest, xSpaceId)

`POST /api/push/subscribe`

Upsert a browser push subscription (endpoint + key material).  Returns `201 Created` on success.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebPushApi();
final SubscribeRequest subscribeRequest = ; // SubscribeRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.subscribe(subscribeRequest, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling WebPushApi->subscribe: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **subscribeRequest** | [**SubscribeRequest**](SubscribeRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **unsubscribe**
> unsubscribe(unsubscribeRequest, xSpaceId)

`DELETE /api/push/subscribe`

Remove a push subscription by its endpoint URL.  Returns `204 No Content` whether or not the subscription existed.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebPushApi();
final UnsubscribeRequest unsubscribeRequest = ; // UnsubscribeRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.unsubscribe(unsubscribeRequest, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling WebPushApi->unsubscribe: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **unsubscribeRequest** | [**UnsubscribeRequest**](UnsubscribeRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **vapidPublicKey**
> VapidKeyResponse vapidPublicKey(xSpaceId)

`GET /api/push/vapid-public-key`

Returns the server's VAPID public key as a base64url-encoded string. The Flutter PWA uses this as the `applicationServerKey` argument to `PushManager.subscribe()`.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWebPushApi();
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.vapidPublicKey(xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WebPushApi->vapidPublicKey: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**VapidKeyResponse**](VapidKeyResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

