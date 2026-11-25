# assistant_api.api.SkillsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createSkill**](SkillsApi.md#createskill) | **POST** /api/skills | &#x60;POST /api/skills&#x60; — create a new user skill.
[**deleteSkill**](SkillsApi.md#deleteskill) | **DELETE** /api/skills/{name} | &#x60;DELETE /api/skills/{name}&#x60; — delete a user skill.
[**getSkill**](SkillsApi.md#getskill) | **GET** /api/skills/{name} | &#x60;GET /api/skills/{name}&#x60; — get a skill by name.
[**listPersonaSkills**](SkillsApi.md#listpersonaskills) | **GET** /api/personas/{persona_id}/skills | &#x60;GET /api/personas/{persona_id}/skills&#x60; — list skills for a persona.
[**listSkills**](SkillsApi.md#listskills) | **GET** /api/skills | &#x60;GET /api/skills&#x60; — list all skills.
[**updateSkill**](SkillsApi.md#updateskill) | **PUT** /api/skills/{name} | &#x60;PUT /api/skills/{name}&#x60; — update a user skill.


# **createSkill**
> SkillDetail createSkill(createSkillRequest, xSpaceId)

`POST /api/skills` — create a new user skill.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getSkillsApi();
final CreateSkillRequest createSkillRequest = ; // CreateSkillRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.createSkill(createSkillRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling SkillsApi->createSkill: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createSkillRequest** | [**CreateSkillRequest**](CreateSkillRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**SkillDetail**](SkillDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteSkill**
> deleteSkill(name, xSpaceId)

`DELETE /api/skills/{name}` — delete a user skill.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getSkillsApi();
final String name = name_example; // String | Skill name
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.deleteSkill(name, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling SkillsApi->deleteSkill: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **name** | **String**| Skill name | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getSkill**
> SkillDetail getSkill(name, xSpaceId)

`GET /api/skills/{name}` — get a skill by name.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getSkillsApi();
final String name = name_example; // String | Skill name
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.getSkill(name, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling SkillsApi->getSkill: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **name** | **String**| Skill name | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**SkillDetail**](SkillDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listPersonaSkills**
> BuiltList<SkillEntryResponse> listPersonaSkills(personaId, xSpaceId)

`GET /api/personas/{persona_id}/skills` — list skills for a persona.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getSkillsApi();
final String personaId = personaId_example; // String | Persona ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.listPersonaSkills(personaId, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling SkillsApi->listPersonaSkills: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **personaId** | **String**| Persona ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**BuiltList&lt;SkillEntryResponse&gt;**](SkillEntryResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listSkills**
> BuiltList<SkillDetail> listSkills(xSpaceId)

`GET /api/skills` — list all skills.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getSkillsApi();
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.listSkills(xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling SkillsApi->listSkills: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**BuiltList&lt;SkillDetail&gt;**](SkillDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateSkill**
> SkillDetail updateSkill(name, updateSkillRequest, xSpaceId)

`PUT /api/skills/{name}` — update a user skill.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getSkillsApi();
final String name = name_example; // String | Skill name
final UpdateSkillRequest updateSkillRequest = ; // UpdateSkillRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.updateSkill(name, updateSkillRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling SkillsApi->updateSkill: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **name** | **String**| Skill name | 
 **updateSkillRequest** | [**UpdateSkillRequest**](UpdateSkillRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**SkillDetail**](SkillDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

