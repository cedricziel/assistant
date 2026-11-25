# assistant_api.api.IntegrationsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**getCatalogEntry**](IntegrationsApi.md#getcatalogentry) | **GET** /api/integrations/catalog/{id} | &#x60;GET /api/integrations/catalog/{id}&#x60; — get one catalog entry.
[**listCatalogEntries**](IntegrationsApi.md#listcatalogentries) | **GET** /api/integrations/catalog | &#x60;GET /api/integrations/catalog&#x60; — search the integrations catalog.


# **getCatalogEntry**
> CatalogEntrySummary getCatalogEntry(id)

`GET /api/integrations/catalog/{id}` — get one catalog entry.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getIntegrationsApi();
final String id = id_example; // String | Catalog entry id, e.g. `curated/linear-app-mcp`. It contains one slash; send it as is or percent-encoded.

try {
    final response = api.getCatalogEntry(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling IntegrationsApi->getCatalogEntry: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Catalog entry id, e.g. `curated/linear-app-mcp`. It contains one slash; send it as is or percent-encoded. | 

### Return type

[**CatalogEntrySummary**](CatalogEntrySummary.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json, text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listCatalogEntries**
> CatalogEntryListResponse listCatalogEntries(q, category, limit, cursor)

`GET /api/integrations/catalog` — search the integrations catalog.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getIntegrationsApi();
final String q = q_example; // String | Case-insensitive text matched against name, domain and description.
final String category = category_example; // String | Only entries in this category (case-insensitive).
final int limit = 56; // int | Page size. Default 50, maximum 200.
final String cursor = cursor_example; // String | `next_cursor` of the previous page.

try {
    final response = api.listCatalogEntries(q, category, limit, cursor);
    print(response);
} on DioException catch (e) {
    print('Exception when calling IntegrationsApi->listCatalogEntries: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **q** | **String**| Case-insensitive text matched against name, domain and description. | [optional] 
 **category** | **String**| Only entries in this category (case-insensitive). | [optional] 
 **limit** | **int**| Page size. Default 50, maximum 200. | [optional] 
 **cursor** | **String**| `next_cursor` of the previous page. | [optional] 

### Return type

[**CatalogEntryListResponse**](CatalogEntryListResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json, text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

