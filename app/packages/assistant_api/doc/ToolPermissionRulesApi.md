# assistant_api.api.ToolPermissionRulesApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**deleteToolPermissionRule**](ToolPermissionRulesApi.md#deletetoolpermissionrule) | **DELETE** /api/tool-permission-rules/{id} | &#x60;DELETE /api/tool-permission-rules/{id}&#x60;
[**listToolPermissionRules**](ToolPermissionRulesApi.md#listtoolpermissionrules) | **GET** /api/tool-permission-rules | &#x60;GET /api/tool-permission-rules&#x60;


# **deleteToolPermissionRule**
> deleteToolPermissionRule(id, xSpaceId)

`DELETE /api/tool-permission-rules/{id}`

Revoke a saved rule. Members can revoke their own rules; a space admin or org admin can revoke any rule in the selected space.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getToolPermissionRulesApi();
final String id = id_example; // String | Rule id
final String xSpaceId = xSpaceId_example; // String | Space the rule belongs to

try {
    api.deleteToolPermissionRule(id, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling ToolPermissionRulesApi->deleteToolPermissionRule: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Rule id | 
 **xSpaceId** | **String**| Space the rule belongs to | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listToolPermissionRules**
> BuiltList<ToolPermissionRuleSummary> listToolPermissionRules(scope, xSpaceId)

`GET /api/tool-permission-rules`

The caller's saved rules in the selected space, oldest first. With `scope=space`, every member's rules (space admins and org admins only).

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getToolPermissionRulesApi();
final RuleScope scope = ; // RuleScope | `own` (default) or `space` (admins only).
final String xSpaceId = xSpaceId_example; // String | Space to read rules from

try {
    final response = api.listToolPermissionRules(scope, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling ToolPermissionRulesApi->listToolPermissionRules: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **scope** | [**RuleScope**](.md)| `own` (default) or `space` (admins only). | [optional] 
 **xSpaceId** | **String**| Space to read rules from | [optional] 

### Return type

[**BuiltList&lt;ToolPermissionRuleSummary&gt;**](ToolPermissionRuleSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

