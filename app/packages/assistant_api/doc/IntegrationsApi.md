# assistant_api.api.IntegrationsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createIntegration**](IntegrationsApi.md#createintegration) | **POST** /api/integrations | &#x60;POST /api/integrations&#x60; — install an integration.
[**deleteIntegration**](IntegrationsApi.md#deleteintegration) | **DELETE** /api/integrations/{id} | &#x60;DELETE /api/integrations/{id}&#x60; — remove an integration and its credentials.
[**deleteSpaceAccess**](IntegrationsApi.md#deletespaceaccess) | **DELETE** /api/integrations/{id}/spaces/{space_id} | &#x60;DELETE /api/integrations/{id}/spaces/{space_id}?mode&#x3D;&#x60; — remove a space row.
[**getCatalogEntry**](IntegrationsApi.md#getcatalogentry) | **GET** /api/integrations/catalog/{id} | &#x60;GET /api/integrations/catalog/{id}&#x60; — get one catalog entry.
[**getIntegration**](IntegrationsApi.md#getintegration) | **GET** /api/integrations/{id} | &#x60;GET /api/integrations/{id}&#x60; — get one integration.
[**getIntegrationPolicy**](IntegrationsApi.md#getintegrationpolicy) | **GET** /api/integrations/policy | &#x60;GET /api/integrations/policy&#x60; — the org&#39;s integration policy.
[**listCatalogEntries**](IntegrationsApi.md#listcatalogentries) | **GET** /api/integrations/catalog | &#x60;GET /api/integrations/catalog&#x60; — search the integrations catalog.
[**listIntegrations**](IntegrationsApi.md#listintegrations) | **GET** /api/integrations | &#x60;GET /api/integrations&#x60; — list the integrations the caller may see.
[**putIntegrationPolicy**](IntegrationsApi.md#putintegrationpolicy) | **PUT** /api/integrations/policy | &#x60;PUT /api/integrations/policy&#x60; — replace the org&#39;s integration policy.
[**putSpaceAccess**](IntegrationsApi.md#putspaceaccess) | **PUT** /api/integrations/{id}/spaces/{space_id} | &#x60;PUT /api/integrations/{id}/spaces/{space_id}&#x60; — add a space row.
[**updateIntegration**](IntegrationsApi.md#updateintegration) | **PATCH** /api/integrations/{id} | &#x60;PATCH /api/integrations/{id}&#x60; — update an integration.


# **createIntegration**
> IntegrationDetail createIntegration(createIntegrationRequest)

`POST /api/integrations` — install an integration.

Org admins install at the org layer; space admins (or org admins) at the space layer; anyone at the user layer. The org policy limits the space and user layers (`403`). A catalog install copies its URL and auth template from the catalog index.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getIntegrationsApi();
final CreateIntegrationRequest createIntegrationRequest = ; // CreateIntegrationRequest | 

try {
    final response = api.createIntegration(createIntegrationRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling IntegrationsApi->createIntegration: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createIntegrationRequest** | [**CreateIntegrationRequest**](CreateIntegrationRequest.md)|  | 

### Return type

[**IntegrationDetail**](IntegrationDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json, text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteIntegration**
> deleteIntegration(id)

`DELETE /api/integrations/{id}` — remove an integration and its credentials.

Org admins may remove any user's personal integration.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getIntegrationsApi();
final String id = id_example; // String | Integration id, `int_<uuid>`

try {
    api.deleteIntegration(id);
} on DioException catch (e) {
    print('Exception when calling IntegrationsApi->deleteIntegration: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Integration id, `int_<uuid>` | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: text/plain, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteSpaceAccess**
> deleteSpaceAccess(id, spaceId, mode)

`DELETE /api/integrations/{id}/spaces/{space_id}?mode=` — remove a space row.

The `mode` query parameter names the row (`allow` or `opt_out`) and has the same permissions as `PUT`. Removing a row that does not exist succeeds.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getIntegrationsApi();
final String id = id_example; // String | Org-layer integration id, `int_<uuid>`
final String spaceId = spaceId_example; // String | Space id
final SpaceAccessMode mode = ; // SpaceAccessMode | Which row to remove.

try {
    api.deleteSpaceAccess(id, spaceId, mode);
} on DioException catch (e) {
    print('Exception when calling IntegrationsApi->deleteSpaceAccess: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Org-layer integration id, `int_<uuid>` | 
 **spaceId** | **String**| Space id | 
 **mode** | [**SpaceAccessMode**](.md)| Which row to remove. | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json, text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

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

# **getIntegration**
> IntegrationDetail getIntegration(id)

`GET /api/integrations/{id}` — get one integration.

`404` when the caller may not see it, including another user's personal integration.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getIntegrationsApi();
final String id = id_example; // String | Integration id, `int_<uuid>`

try {
    final response = api.getIntegration(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling IntegrationsApi->getIntegration: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Integration id, `int_<uuid>` | 

### Return type

[**IntegrationDetail**](IntegrationDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json, text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getIntegrationPolicy**
> IntegrationPolicyResponse getIntegrationPolicy()

`GET /api/integrations/policy` — the org's integration policy.

Open to every member: the install flow needs to explain what is allowed.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getIntegrationsApi();

try {
    final response = api.getIntegrationPolicy();
    print(response);
} on DioException catch (e) {
    print('Exception when calling IntegrationsApi->getIntegrationPolicy: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**IntegrationPolicyResponse**](IntegrationPolicyResponse.md)

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

# **listIntegrations**
> IntegrationListResponse listIntegrations(layer, spaceId)

`GET /api/integrations` — list the integrations the caller may see.

Members see the org-layer integrations offered to their spaces, the integrations of spaces they belong to, and their own user-layer ones. Org admins see everything, including other users' integrations (without credential material).

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getIntegrationsApi();
final IntegrationLayer layer = ; // IntegrationLayer | Only integrations of this layer.
final String spaceId = spaceId_example; // String | Only integrations in use in this space: its own space-layer ones, plus the org-layer ones offered to it that it has not opted out of. User-layer integrations belong to no space and are never returned. Without this filter, opted-out org integrations are listed too, so a space can opt back in.

try {
    final response = api.listIntegrations(layer, spaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling IntegrationsApi->listIntegrations: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **layer** | [**IntegrationLayer**](.md)| Only integrations of this layer. | [optional] 
 **spaceId** | **String**| Only integrations in use in this space: its own space-layer ones, plus the org-layer ones offered to it that it has not opted out of. User-layer integrations belong to no space and are never returned. Without this filter, opted-out org integrations are listed too, so a space can opt back in. | [optional] 

### Return type

[**IntegrationListResponse**](IntegrationListResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json, text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **putIntegrationPolicy**
> IntegrationPolicyResponse putIntegrationPolicy(updateIntegrationPolicyRequest)

`PUT /api/integrations/policy` — replace the org's integration policy.

Org admins only. Tightening it deletes nothing: integrations that no longer comply report status `blocked_by_policy` and contribute no tools.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getIntegrationsApi();
final UpdateIntegrationPolicyRequest updateIntegrationPolicyRequest = ; // UpdateIntegrationPolicyRequest | 

try {
    final response = api.putIntegrationPolicy(updateIntegrationPolicyRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling IntegrationsApi->putIntegrationPolicy: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **updateIntegrationPolicyRequest** | [**UpdateIntegrationPolicyRequest**](UpdateIntegrationPolicyRequest.md)|  | 

### Return type

[**IntegrationPolicyResponse**](IntegrationPolicyResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json, text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **putSpaceAccess**
> SpaceAccessEntry putSpaceAccess(id, spaceId, setSpaceAccessRequest)

`PUT /api/integrations/{id}/spaces/{space_id}` — add a space row.

`mode: allow` (org admins) restricts the integration to the spaces that have an allow row. `mode: opt_out` (admins of that space, or org admins) removes it from that space.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getIntegrationsApi();
final String id = id_example; // String | Org-layer integration id, `int_<uuid>`
final String spaceId = spaceId_example; // String | Space id
final SetSpaceAccessRequest setSpaceAccessRequest = ; // SetSpaceAccessRequest | 

try {
    final response = api.putSpaceAccess(id, spaceId, setSpaceAccessRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling IntegrationsApi->putSpaceAccess: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Org-layer integration id, `int_<uuid>` | 
 **spaceId** | **String**| Space id | 
 **setSpaceAccessRequest** | [**SetSpaceAccessRequest**](SetSpaceAccessRequest.md)|  | 

### Return type

[**SpaceAccessEntry**](SpaceAccessEntry.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json, text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateIntegration**
> IntegrationDetail updateIntegration(id, updateIntegrationRequest)

`PATCH /api/integrations/{id}` — update an integration.

Layer, owner, catalog entry, auth kind and credential mode are fixed at install time. A catalog install cannot change its URL or header template; moving a custom URL deletes its stored credentials.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getIntegrationsApi();
final String id = id_example; // String | Integration id, `int_<uuid>`
final UpdateIntegrationRequest updateIntegrationRequest = ; // UpdateIntegrationRequest | 

try {
    final response = api.updateIntegration(id, updateIntegrationRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling IntegrationsApi->updateIntegration: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Integration id, `int_<uuid>` | 
 **updateIntegrationRequest** | [**UpdateIntegrationRequest**](UpdateIntegrationRequest.md)|  | 

### Return type

[**IntegrationDetail**](IntegrationDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json, text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

