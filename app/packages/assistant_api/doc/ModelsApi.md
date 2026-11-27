# assistant_api.api.ModelsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**listModels**](ModelsApi.md#listmodels) | **GET** /api/models | &#x60;GET /api/models&#x60; — list the models a reply may be regenerated with.


# **listModels**
> BuiltList<ModelSummary> listModels(xSpaceId)

`GET /api/models` — list the models a reply may be regenerated with.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getModelsApi();
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.listModels(xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ModelsApi->listModels: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**BuiltList&lt;ModelSummary&gt;**](ModelSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

