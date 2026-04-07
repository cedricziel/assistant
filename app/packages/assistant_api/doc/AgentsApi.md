# assistant_api.api.AgentsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**deleteAgent**](AgentsApi.md#deleteagent) | **DELETE** /api/agents/{id} | &#x60;DELETE /api/agents/{id}&#x60; — remove a registered agent.
[**getAgent**](AgentsApi.md#getagent) | **GET** /api/agents/{id} | &#x60;GET /api/agents/{id}&#x60; — get an agent by ID.
[**listAgents**](AgentsApi.md#listagents) | **GET** /api/agents | &#x60;GET /api/agents&#x60; — list all registered agents.
[**registerAgent**](AgentsApi.md#registeragent) | **POST** /api/agents | &#x60;POST /api/agents&#x60; — register a new agent.
[**setDefaultAgent**](AgentsApi.md#setdefaultagent) | **POST** /api/agents/{id}/set-default | &#x60;POST /api/agents/{id}/set-default&#x60; — set an agent as the default.
[**updateAgent**](AgentsApi.md#updateagent) | **PUT** /api/agents/{id} | &#x60;PUT /api/agents/{id}&#x60; — update an agent&#39;s card.


# **deleteAgent**
> deleteAgent(id)

`DELETE /api/agents/{id}` — remove a registered agent.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getAgentsApi();
final String id = id_example; // String | Agent ID

try {
    api.deleteAgent(id);
} on DioException catch (e) {
    print('Exception when calling AgentsApi->deleteAgent: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Agent ID | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getAgent**
> AgentDetail getAgent(id)

`GET /api/agents/{id}` — get an agent by ID.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getAgentsApi();
final String id = id_example; // String | Agent ID

try {
    final response = api.getAgent(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling AgentsApi->getAgent: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Agent ID | 

### Return type

[**AgentDetail**](AgentDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listAgents**
> BuiltList<AgentSummary> listAgents()

`GET /api/agents` — list all registered agents.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getAgentsApi();

try {
    final response = api.listAgents();
    print(response);
} on DioException catch (e) {
    print('Exception when calling AgentsApi->listAgents: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**BuiltList&lt;AgentSummary&gt;**](AgentSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **registerAgent**
> AgentDetail registerAgent(registerAgentRequest)

`POST /api/agents` — register a new agent.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getAgentsApi();
final RegisterAgentRequest registerAgentRequest = ; // RegisterAgentRequest | 

try {
    final response = api.registerAgent(registerAgentRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling AgentsApi->registerAgent: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **registerAgentRequest** | [**RegisterAgentRequest**](RegisterAgentRequest.md)|  | 

### Return type

[**AgentDetail**](AgentDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **setDefaultAgent**
> AgentDetail setDefaultAgent(id)

`POST /api/agents/{id}/set-default` — set an agent as the default.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getAgentsApi();
final String id = id_example; // String | Agent ID

try {
    final response = api.setDefaultAgent(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling AgentsApi->setDefaultAgent: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Agent ID | 

### Return type

[**AgentDetail**](AgentDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateAgent**
> AgentDetail updateAgent(id, updateAgentRequest)

`PUT /api/agents/{id}` — update an agent's card.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getAgentsApi();
final String id = id_example; // String | Agent ID
final UpdateAgentRequest updateAgentRequest = ; // UpdateAgentRequest | 

try {
    final response = api.updateAgent(id, updateAgentRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling AgentsApi->updateAgent: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Agent ID | 
 **updateAgentRequest** | [**UpdateAgentRequest**](UpdateAgentRequest.md)|  | 

### Return type

[**AgentDetail**](AgentDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

