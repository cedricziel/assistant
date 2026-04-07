# assistant_api.api.WorkflowsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**activateWorkflow**](WorkflowsApi.md#activateworkflow) | **POST** /api/workflows/{id}/activate | &#x60;POST /api/workflows/{id}/activate&#x60; — activate a workflow.
[**createWorkflow**](WorkflowsApi.md#createworkflow) | **POST** /api/workflows | &#x60;POST /api/workflows&#x60; — create a new workflow.
[**deactivateWorkflow**](WorkflowsApi.md#deactivateworkflow) | **POST** /api/workflows/{id}/deactivate | &#x60;POST /api/workflows/{id}/deactivate&#x60; — deactivate a workflow.
[**deleteWorkflow**](WorkflowsApi.md#deleteworkflow) | **DELETE** /api/workflows/{id} | &#x60;DELETE /api/workflows/{id}&#x60; — delete a workflow.
[**getWorkflow**](WorkflowsApi.md#getworkflow) | **GET** /api/workflows/{id} | &#x60;GET /api/workflows/{id}&#x60; — fetch a workflow.
[**getWorkflowRun**](WorkflowsApi.md#getworkflowrun) | **GET** /api/workflows/{id}/runs/{run_id} | &#x60;GET /api/workflows/{id}/runs/{run_id}&#x60; — workflow run detail with steps.
[**getWorkflowWebhookSecrets**](WorkflowsApi.md#getworkflowwebhooksecrets) | **GET** /api/workflows/{id}/webhook-secrets | &#x60;GET /api/workflows/{id}/webhook-secrets&#x60; — reveal webhook URL and token.
[**listWorkflowRuns**](WorkflowsApi.md#listworkflowruns) | **GET** /api/workflows/{id}/runs | &#x60;GET /api/workflows/{id}/runs&#x60; — list recent runs (up to 50).
[**listWorkflows**](WorkflowsApi.md#listworkflows) | **GET** /api/workflows | &#x60;GET /api/workflows&#x60; — list all workflows.
[**testRunWorkflow**](WorkflowsApi.md#testrunworkflow) | **POST** /api/workflows/{id}/test-run | &#x60;POST /api/workflows/{id}/test-run&#x60; — queue a manual test run.
[**updateWorkflow**](WorkflowsApi.md#updateworkflow) | **PUT** /api/workflows/{id} | &#x60;PUT /api/workflows/{id}&#x60; — update a workflow.


# **activateWorkflow**
> WorkflowDetail activateWorkflow(id)

`POST /api/workflows/{id}/activate` — activate a workflow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID

try {
    final response = api.activateWorkflow(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->activateWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 

### Return type

[**WorkflowDetail**](WorkflowDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createWorkflow**
> WorkflowDetail createWorkflow(workflowUpsertRequest)

`POST /api/workflows` — create a new workflow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final WorkflowUpsertRequest workflowUpsertRequest = ; // WorkflowUpsertRequest | 

try {
    final response = api.createWorkflow(workflowUpsertRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->createWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workflowUpsertRequest** | [**WorkflowUpsertRequest**](WorkflowUpsertRequest.md)|  | 

### Return type

[**WorkflowDetail**](WorkflowDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deactivateWorkflow**
> WorkflowDetail deactivateWorkflow(id)

`POST /api/workflows/{id}/deactivate` — deactivate a workflow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID

try {
    final response = api.deactivateWorkflow(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->deactivateWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 

### Return type

[**WorkflowDetail**](WorkflowDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteWorkflow**
> deleteWorkflow(id)

`DELETE /api/workflows/{id}` — delete a workflow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID

try {
    api.deleteWorkflow(id);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->deleteWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getWorkflow**
> WorkflowDetail getWorkflow(id)

`GET /api/workflows/{id}` — fetch a workflow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID

try {
    final response = api.getWorkflow(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->getWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 

### Return type

[**WorkflowDetail**](WorkflowDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getWorkflowRun**
> WorkflowRunDetail getWorkflowRun(id, runId)

`GET /api/workflows/{id}/runs/{run_id}` — workflow run detail with steps.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID
final String runId = runId_example; // String | Run UUID

try {
    final response = api.getWorkflowRun(id, runId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->getWorkflowRun: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 
 **runId** | **String**| Run UUID | 

### Return type

[**WorkflowRunDetail**](WorkflowRunDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getWorkflowWebhookSecrets**
> WorkflowWebhookSecrets getWorkflowWebhookSecrets(id)

`GET /api/workflows/{id}/webhook-secrets` — reveal webhook URL and token.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID

try {
    final response = api.getWorkflowWebhookSecrets(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->getWorkflowWebhookSecrets: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 

### Return type

[**WorkflowWebhookSecrets**](WorkflowWebhookSecrets.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listWorkflowRuns**
> BuiltList<WorkflowRunSummary> listWorkflowRuns(id)

`GET /api/workflows/{id}/runs` — list recent runs (up to 50).

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID

try {
    final response = api.listWorkflowRuns(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->listWorkflowRuns: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 

### Return type

[**BuiltList&lt;WorkflowRunSummary&gt;**](WorkflowRunSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listWorkflows**
> BuiltList<WorkflowSummary> listWorkflows()

`GET /api/workflows` — list all workflows.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();

try {
    final response = api.listWorkflows();
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->listWorkflows: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**BuiltList&lt;WorkflowSummary&gt;**](WorkflowSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **testRunWorkflow**
> WorkflowRunPreview testRunWorkflow(id)

`POST /api/workflows/{id}/test-run` — queue a manual test run.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID

try {
    final response = api.testRunWorkflow(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->testRunWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 

### Return type

[**WorkflowRunPreview**](WorkflowRunPreview.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateWorkflow**
> WorkflowDetail updateWorkflow(id, workflowUpsertRequest)

`PUT /api/workflows/{id}` — update a workflow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID
final WorkflowUpsertRequest workflowUpsertRequest = ; // WorkflowUpsertRequest | 

try {
    final response = api.updateWorkflow(id, workflowUpsertRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->updateWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 
 **workflowUpsertRequest** | [**WorkflowUpsertRequest**](WorkflowUpsertRequest.md)|  | 

### Return type

[**WorkflowDetail**](WorkflowDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

