# assistant_api.api.LogsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**listLogs**](LogsApi.md#listlogs) | **GET** /api/logs | &#x60;GET /api/logs&#x60; — list recent log entries, newest first.


# **listLogs**
> BuiltList<LogEntryResponse> listLogs(limit, offset, search, severity, since, until, traceId, conversation)

`GET /api/logs` — list recent log entries, newest first.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getLogsApi();
final int limit = 789; // int | 
final int offset = 789; // int | 
final String search = search_example; // String | 
final String severity = severity_example; // String | 
final DateTime since = 2013-10-20T19:20:30+01:00; // DateTime | 
final DateTime until = 2013-10-20T19:20:30+01:00; // DateTime | 
final String traceId = traceId_example; // String | 
final String conversation = conversation_example; // String | 

try {
    final response = api.listLogs(limit, offset, search, severity, since, until, traceId, conversation);
    print(response);
} on DioException catch (e) {
    print('Exception when calling LogsApi->listLogs: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **limit** | **int**|  | [optional] 
 **offset** | **int**|  | [optional] 
 **search** | **String**|  | [optional] 
 **severity** | **String**|  | [optional] 
 **since** | **DateTime**|  | [optional] 
 **until** | **DateTime**|  | [optional] 
 **traceId** | **String**|  | [optional] 
 **conversation** | **String**|  | [optional] 

### Return type

[**BuiltList&lt;LogEntryResponse&gt;**](LogEntryResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

