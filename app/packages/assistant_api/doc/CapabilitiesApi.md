# assistant_api.api.CapabilitiesApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**getCapabilities**](CapabilitiesApi.md#getcapabilities) | **GET** /api/capabilities | &#x60;GET /api/capabilities&#x60; — return server capability flags.


# **getCapabilities**
> ServerCapabilities getCapabilities()

`GET /api/capabilities` — return server capability flags.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getCapabilitiesApi();

try {
    final response = api.getCapabilities();
    print(response);
} on DioException catch (e) {
    print('Exception when calling CapabilitiesApi->getCapabilities: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**ServerCapabilities**](ServerCapabilities.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

