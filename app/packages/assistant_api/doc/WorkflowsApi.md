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
[**publicWebhookTrigger**](WorkflowsApi.md#publicwebhooktrigger) | **POST** /workflow-hooks/{org_slug}/{space_slug}/{id}/{token} | Public webhook trigger endpoint — no auth required.
[**testRunWorkflow**](WorkflowsApi.md#testrunworkflow) | **POST** /api/workflows/{id}/test-run | &#x60;POST /api/workflows/{id}/test-run&#x60; — queue a manual test run.
[**updateWorkflow**](WorkflowsApi.md#updateworkflow) | **PUT** /api/workflows/{id} | &#x60;PUT /api/workflows/{id}&#x60; — update a workflow.


# **activateWorkflow**
> WorkflowDetail activateWorkflow(id, xSpaceId)

`POST /api/workflows/{id}/activate` — activate a workflow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.activateWorkflow(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->activateWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**WorkflowDetail**](WorkflowDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createWorkflow**
> WorkflowDetail createWorkflow(workflowUpsertRequest, xSpaceId)

`POST /api/workflows` — create a new workflow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final WorkflowUpsertRequest workflowUpsertRequest = ; // WorkflowUpsertRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.createWorkflow(workflowUpsertRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->createWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workflowUpsertRequest** | [**WorkflowUpsertRequest**](WorkflowUpsertRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**WorkflowDetail**](WorkflowDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deactivateWorkflow**
> WorkflowDetail deactivateWorkflow(id, xSpaceId)

`POST /api/workflows/{id}/deactivate` — deactivate a workflow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.deactivateWorkflow(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->deactivateWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**WorkflowDetail**](WorkflowDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteWorkflow**
> deleteWorkflow(id, xSpaceId)

`DELETE /api/workflows/{id}` — delete a workflow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.deleteWorkflow(id, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->deleteWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getWorkflow**
> WorkflowDetail getWorkflow(id, xSpaceId)

`GET /api/workflows/{id}` — fetch a workflow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.getWorkflow(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->getWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**WorkflowDetail**](WorkflowDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getWorkflowRun**
> WorkflowRunDetail getWorkflowRun(id, runId, xSpaceId)

`GET /api/workflows/{id}/runs/{run_id}` — workflow run detail with steps.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID
final String runId = runId_example; // String | Run UUID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.getWorkflowRun(id, runId, xSpaceId);
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
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**WorkflowRunDetail**](WorkflowRunDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getWorkflowWebhookSecrets**
> WorkflowWebhookSecrets getWorkflowWebhookSecrets(id, xSpaceId)

`GET /api/workflows/{id}/webhook-secrets` — reveal webhook URL and token.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.getWorkflowWebhookSecrets(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->getWorkflowWebhookSecrets: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**WorkflowWebhookSecrets**](WorkflowWebhookSecrets.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listWorkflowRuns**
> BuiltList<WorkflowRunSummary> listWorkflowRuns(id, xSpaceId)

`GET /api/workflows/{id}/runs` — list recent runs (up to 50).

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.listWorkflowRuns(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->listWorkflowRuns: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**BuiltList&lt;WorkflowRunSummary&gt;**](WorkflowRunSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listWorkflows**
> BuiltList<WorkflowSummary> listWorkflows(xSpaceId)

`GET /api/workflows` — list all workflows.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.listWorkflows(xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->listWorkflows: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**BuiltList&lt;WorkflowSummary&gt;**](WorkflowSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **publicWebhookTrigger**
> WorkflowWebhookTriggerAccepted publicWebhookTrigger(orgSlug, spaceSlug, id, token, body)

Public webhook trigger endpoint — no auth required.

Receives an inbound HTTP POST from an external system and queues a workflow run with trigger type `Webhook`.  The `token` path parameter is the per-workflow HMAC secret that acts as authentication.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String orgSlug = orgSlug_example; // String | Slug of the org that owns the workflow's space
final String spaceSlug = spaceSlug_example; // String | Slug of the space that owns the workflow
final String id = id_example; // String | Workflow ID
final String token = token_example; // String | Webhook HMAC token
final JsonObject body = ; // JsonObject | 

try {
    final response = api.publicWebhookTrigger(orgSlug, spaceSlug, id, token, body);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->publicWebhookTrigger: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgSlug** | **String**| Slug of the org that owns the workflow's space | 
 **spaceSlug** | **String**| Slug of the space that owns the workflow | 
 **id** | **String**| Workflow ID | 
 **token** | **String**| Webhook HMAC token | 
 **body** | **JsonObject**|  | 

### Return type

[**WorkflowWebhookTriggerAccepted**](WorkflowWebhookTriggerAccepted.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **testRunWorkflow**
> WorkflowRunPreview testRunWorkflow(id, xSpaceId)

`POST /api/workflows/{id}/test-run` — queue a manual test run.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.testRunWorkflow(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling WorkflowsApi->testRunWorkflow: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Workflow UUID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**WorkflowRunPreview**](WorkflowRunPreview.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateWorkflow**
> WorkflowDetail updateWorkflow(id, workflowUpsertRequest, xSpaceId)

`PUT /api/workflows/{id}` — update a workflow.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getWorkflowsApi();
final String id = id_example; // String | Workflow UUID
final WorkflowUpsertRequest workflowUpsertRequest = ; // WorkflowUpsertRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.updateWorkflow(id, workflowUpsertRequest, xSpaceId);
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
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**WorkflowDetail**](WorkflowDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

