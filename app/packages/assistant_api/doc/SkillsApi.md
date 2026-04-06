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
> SkillDetail createSkill(createSkillRequest)

`POST /api/skills` — create a new user skill.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getSkillsApi();
final CreateSkillRequest createSkillRequest = ; // CreateSkillRequest | 

try {
    final response = api.createSkill(createSkillRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling SkillsApi->createSkill: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createSkillRequest** | [**CreateSkillRequest**](CreateSkillRequest.md)|  | 

### Return type

[**SkillDetail**](SkillDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteSkill**
> deleteSkill(name)

`DELETE /api/skills/{name}` — delete a user skill.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getSkillsApi();
final String name = name_example; // String | Skill name

try {
    api.deleteSkill(name);
} on DioException catch (e) {
    print('Exception when calling SkillsApi->deleteSkill: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **name** | **String**| Skill name | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getSkill**
> SkillDetail getSkill(name)

`GET /api/skills/{name}` — get a skill by name.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getSkillsApi();
final String name = name_example; // String | Skill name

try {
    final response = api.getSkill(name);
    print(response);
} on DioException catch (e) {
    print('Exception when calling SkillsApi->getSkill: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **name** | **String**| Skill name | 

### Return type

[**SkillDetail**](SkillDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listPersonaSkills**
> BuiltList<SkillEntryResponse> listPersonaSkills(personaId)

`GET /api/personas/{persona_id}/skills` — list skills for a persona.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getSkillsApi();
final String personaId = personaId_example; // String | Persona ID

try {
    final response = api.listPersonaSkills(personaId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling SkillsApi->listPersonaSkills: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **personaId** | **String**| Persona ID | 

### Return type

[**BuiltList&lt;SkillEntryResponse&gt;**](SkillEntryResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listSkills**
> BuiltList<SkillDetail> listSkills()

`GET /api/skills` — list all skills.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getSkillsApi();

try {
    final response = api.listSkills();
    print(response);
} on DioException catch (e) {
    print('Exception when calling SkillsApi->listSkills: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**BuiltList&lt;SkillDetail&gt;**](SkillDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateSkill**
> SkillDetail updateSkill(name, updateSkillRequest)

`PUT /api/skills/{name}` — update a user skill.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getSkillsApi();
final String name = name_example; // String | Skill name
final UpdateSkillRequest updateSkillRequest = ; // UpdateSkillRequest | 

try {
    final response = api.updateSkill(name, updateSkillRequest);
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

### Return type

[**SkillDetail**](SkillDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

