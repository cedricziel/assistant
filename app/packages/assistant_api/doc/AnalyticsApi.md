# assistant_api.api.AnalyticsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**getAnalytics**](AnalyticsApi.md#getanalytics) | **GET** /api/analytics | &#x60;GET /api/analytics&#x60; — get aggregated usage analytics for a time window.


# **getAnalytics**
> AnalyticsSummaryResponse getAnalytics(window)

`GET /api/analytics` — get aggregated usage analytics for a time window.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getAnalyticsApi();
final int window = 789; // int | Window in hours. Valid values: 1, 6, 24, 72, 168. Defaults to 24.

try {
    final response = api.getAnalytics(window);
    print(response);
} on DioException catch (e) {
    print('Exception when calling AnalyticsApi->getAnalytics: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **window** | **int**| Window in hours. Valid values: 1, 6, 24, 72, 168. Defaults to 24. | [optional] 

### Return type

[**AnalyticsSummaryResponse**](AnalyticsSummaryResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

