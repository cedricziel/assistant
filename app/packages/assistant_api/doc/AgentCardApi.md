# assistant_api.api.AgentCardApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**getAgentCardWellKnown**](AgentCardApi.md#getagentcardwellknown) | **GET** /.well-known/agent.json | &#x60;GET /.well-known/agent.json&#x60; -- Returns the public agent card.
[**getExtendedAgentCard**](AgentCardApi.md#getextendedagentcard) | **GET** /agent/authenticatedExtendedCard | &#x60;GET /agent/authenticatedExtendedCard&#x60; -- Returns the extended agent card (same as public for now).


# **getAgentCardWellKnown**
> AgentCard getAgentCardWellKnown()

`GET /.well-known/agent.json` -- Returns the public agent card.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getAgentCardApi();

try {
    final response = api.getAgentCardWellKnown();
    print(response);
} on DioException catch (e) {
    print('Exception when calling AgentCardApi->getAgentCardWellKnown: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**AgentCard**](AgentCard.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getExtendedAgentCard**
> AgentCard getExtendedAgentCard()

`GET /agent/authenticatedExtendedCard` -- Returns the extended agent card (same as public for now).

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getAgentCardApi();

try {
    final response = api.getExtendedAgentCard();
    print(response);
} on DioException catch (e) {
    print('Exception when calling AgentCardApi->getExtendedAgentCard: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**AgentCard**](AgentCard.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

