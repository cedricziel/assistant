# assistant_api.api.OrgsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createOrg**](OrgsApi.md#createorg) | **POST** /api/orgs | &#x60;POST /api/orgs&#x60; — create a new organization.
[**getOrg**](OrgsApi.md#getorg) | **GET** /api/orgs/{id} | &#x60;GET /api/orgs/{id}&#x60; — get organization detail.
[**listOrgs**](OrgsApi.md#listorgs) | **GET** /api/orgs | &#x60;GET /api/orgs&#x60; — list organizations the user has access to.
[**updateOrg**](OrgsApi.md#updateorg) | **PATCH** /api/orgs/{id} | &#x60;PATCH /api/orgs/{id}&#x60; — update organization settings.


# **createOrg**
> OrgDetail createOrg(createOrgRequest)

`POST /api/orgs` — create a new organization.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOrgsApi();
final CreateOrgRequest createOrgRequest = ; // CreateOrgRequest | 

try {
    final response = api.createOrg(createOrgRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling OrgsApi->createOrg: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createOrgRequest** | [**CreateOrgRequest**](CreateOrgRequest.md)|  | 

### Return type

[**OrgDetail**](OrgDetail.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getOrg**
> OrgDetail getOrg(id)

`GET /api/orgs/{id}` — get organization detail.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOrgsApi();
final String id = id_example; // String | Organization ID

try {
    final response = api.getOrg(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling OrgsApi->getOrg: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Organization ID | 

### Return type

[**OrgDetail**](OrgDetail.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listOrgs**
> BuiltList<OrgSummary> listOrgs()

`GET /api/orgs` — list organizations the user has access to.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOrgsApi();

try {
    final response = api.listOrgs();
    print(response);
} on DioException catch (e) {
    print('Exception when calling OrgsApi->listOrgs: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**BuiltList&lt;OrgSummary&gt;**](OrgSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateOrg**
> OrgDetail updateOrg(id, updateOrgRequest)

`PATCH /api/orgs/{id}` — update organization settings.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getOrgsApi();
final String id = id_example; // String | Organization ID
final UpdateOrgRequest updateOrgRequest = ; // UpdateOrgRequest | 

try {
    final response = api.updateOrg(id, updateOrgRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling OrgsApi->updateOrg: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Organization ID | 
 **updateOrgRequest** | [**UpdateOrgRequest**](UpdateOrgRequest.md)|  | 

### Return type

[**OrgDetail**](OrgDetail.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

