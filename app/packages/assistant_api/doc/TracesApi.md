# assistant_api.api.TracesApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**getTrace**](TracesApi.md#gettrace) | **GET** /api/traces/{trace_id} | &#x60;GET /api/traces/{trace_id}&#x60; — get a single trace with span breakdown.
[**listTraces**](TracesApi.md#listtraces) | **GET** /api/traces | &#x60;GET /api/traces&#x60; — list recent traces, newest first.


# **getTrace**
> TraceDetailResponse getTrace(traceId)

`GET /api/traces/{trace_id}` — get a single trace with span breakdown.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getTracesApi();
final String traceId = traceId_example; // String | Trace ID

try {
    final response = api.getTrace(traceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling TracesApi->getTrace: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **traceId** | **String**| Trace ID | 

### Return type

[**TraceDetailResponse**](TraceDetailResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listTraces**
> BuiltList<TraceSummaryResponse> listTraces(limit, offset, since, until, skill, status, conversation)

`GET /api/traces` — list recent traces, newest first.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getTracesApi();
final int limit = 789; // int | 
final int offset = 789; // int | 
final DateTime since = 2013-10-20T19:20:30+01:00; // DateTime | 
final DateTime until = 2013-10-20T19:20:30+01:00; // DateTime | 
final String skill = skill_example; // String | 
final String status = status_example; // String | 
final String conversation = conversation_example; // String | 

try {
    final response = api.listTraces(limit, offset, since, until, skill, status, conversation);
    print(response);
} on DioException catch (e) {
    print('Exception when calling TracesApi->listTraces: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **limit** | **int**|  | [optional] 
 **offset** | **int**|  | [optional] 
 **since** | **DateTime**|  | [optional] 
 **until** | **DateTime**|  | [optional] 
 **skill** | **String**|  | [optional] 
 **status** | **String**|  | [optional] 
 **conversation** | **String**|  | [optional] 

### Return type

[**BuiltList&lt;TraceSummaryResponse&gt;**](TraceSummaryResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

