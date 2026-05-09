# assistant_api.api.CatalogApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createSubscription**](CatalogApi.md#createsubscription) | **POST** /api/orgs/{org_id}/spaces/{space_id}/subscriptions | &#x60;POST /api/orgs/{org_id}/spaces/{space_id}/subscriptions&#x60; — subscribe a space to a catalog item.
[**deleteCatalogItem**](CatalogApi.md#deletecatalogitem) | **DELETE** /api/orgs/{org_id}/catalog/{item_id} | &#x60;DELETE /api/orgs/{org_id}/catalog/{item_id}&#x60; — remove from catalog.
[**deleteSubscription**](CatalogApi.md#deletesubscription) | **DELETE** /api/orgs/{org_id}/spaces/{space_id}/subscriptions/{sub_id} | &#x60;DELETE /api/orgs/{org_id}/spaces/{space_id}/subscriptions/{sub_id}&#x60; — unsubscribe.
[**listCatalog**](CatalogApi.md#listcatalog) | **GET** /api/orgs/{org_id}/catalog | &#x60;GET /api/orgs/{org_id}/catalog&#x60; — list all catalog items.
[**listCatalogByType**](CatalogApi.md#listcatalogbytype) | **GET** /api/orgs/{org_id}/catalog/type/{type} | &#x60;GET /api/orgs/{org_id}/catalog/type/{type}&#x60; — list catalog items by type.
[**listSubscriptions**](CatalogApi.md#listsubscriptions) | **GET** /api/orgs/{org_id}/spaces/{space_id}/subscriptions | &#x60;GET /api/orgs/{org_id}/spaces/{space_id}/subscriptions&#x60; — list subscriptions.
[**publishCatalogItem**](CatalogApi.md#publishcatalogitem) | **POST** /api/orgs/{org_id}/catalog | &#x60;POST /api/orgs/{org_id}/catalog&#x60; — publish a resource to the org catalog.


# **createSubscription**
> SubscriptionResponse createSubscription(orgId, spaceId, createSubscriptionRequest)

`POST /api/orgs/{org_id}/spaces/{space_id}/subscriptions` — subscribe a space to a catalog item.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getCatalogApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID
final CreateSubscriptionRequest createSubscriptionRequest = ; // CreateSubscriptionRequest | 

try {
    final response = api.createSubscription(orgId, spaceId, createSubscriptionRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling CatalogApi->createSubscription: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 
 **createSubscriptionRequest** | [**CreateSubscriptionRequest**](CreateSubscriptionRequest.md)|  | 

### Return type

[**SubscriptionResponse**](SubscriptionResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteCatalogItem**
> deleteCatalogItem(orgId, itemId)

`DELETE /api/orgs/{org_id}/catalog/{item_id}` — remove from catalog.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getCatalogApi();
final String orgId = orgId_example; // String | Organization ID
final String itemId = itemId_example; // String | Catalog item ID

try {
    api.deleteCatalogItem(orgId, itemId);
} on DioException catch (e) {
    print('Exception when calling CatalogApi->deleteCatalogItem: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **itemId** | **String**| Catalog item ID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteSubscription**
> deleteSubscription(orgId, spaceId, subId)

`DELETE /api/orgs/{org_id}/spaces/{space_id}/subscriptions/{sub_id}` — unsubscribe.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getCatalogApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID
final String subId = subId_example; // String | Subscription ID

try {
    api.deleteSubscription(orgId, spaceId, subId);
} on DioException catch (e) {
    print('Exception when calling CatalogApi->deleteSubscription: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 
 **subId** | **String**| Subscription ID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listCatalog**
> BuiltList<CatalogItemResponse> listCatalog(orgId)

`GET /api/orgs/{org_id}/catalog` — list all catalog items.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getCatalogApi();
final String orgId = orgId_example; // String | Organization ID

try {
    final response = api.listCatalog(orgId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling CatalogApi->listCatalog: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 

### Return type

[**BuiltList&lt;CatalogItemResponse&gt;**](CatalogItemResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listCatalogByType**
> BuiltList<CatalogItemResponse> listCatalogByType(orgId, type)

`GET /api/orgs/{org_id}/catalog/type/{type}` — list catalog items by type.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getCatalogApi();
final String orgId = orgId_example; // String | Organization ID
final String type = type_example; // String | Resource type: skill, template, interface

try {
    final response = api.listCatalogByType(orgId, type);
    print(response);
} on DioException catch (e) {
    print('Exception when calling CatalogApi->listCatalogByType: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **type** | **String**| Resource type: skill, template, interface | 

### Return type

[**BuiltList&lt;CatalogItemResponse&gt;**](CatalogItemResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listSubscriptions**
> BuiltList<SubscriptionResponse> listSubscriptions(orgId, spaceId)

`GET /api/orgs/{org_id}/spaces/{space_id}/subscriptions` — list subscriptions.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getCatalogApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID

try {
    final response = api.listSubscriptions(orgId, spaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling CatalogApi->listSubscriptions: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 

### Return type

[**BuiltList&lt;SubscriptionResponse&gt;**](SubscriptionResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **publishCatalogItem**
> CatalogItemResponse publishCatalogItem(orgId, publishCatalogItemRequest)

`POST /api/orgs/{org_id}/catalog` — publish a resource to the org catalog.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getCatalogApi();
final String orgId = orgId_example; // String | Organization ID
final PublishCatalogItemRequest publishCatalogItemRequest = ; // PublishCatalogItemRequest | 

try {
    final response = api.publishCatalogItem(orgId, publishCatalogItemRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling CatalogApi->publishCatalogItem: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **publishCatalogItemRequest** | [**PublishCatalogItemRequest**](PublishCatalogItemRequest.md)|  | 

### Return type

[**CatalogItemResponse**](CatalogItemResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

