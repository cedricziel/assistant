# assistant_api.api.PersonasApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**addSkillAccess**](PersonasApi.md#addskillaccess) | **POST** /api/personas/{id}/skill-access/skills | &#x60;POST /api/personas/{id}/skill-access/skills&#x60; — add a skill to the access list.
[**createPersona**](PersonasApi.md#createpersona) | **POST** /api/personas | &#x60;POST /api/personas&#x60; — create a new persona.
[**deleteSkillAccess**](PersonasApi.md#deleteskillaccess) | **DELETE** /api/personas/{id}/skill-access/skills/{skill_name} | &#x60;DELETE /api/personas/{id}/skill-access/skills/{skill_name}&#x60; — remove a skill from the access list.
[**getPersona**](PersonasApi.md#getpersona) | **GET** /api/personas/{id} | &#x60;GET /api/personas/{id}&#x60; — get full persona detail.
[**getPersonaFile**](PersonasApi.md#getpersonafile) | **GET** /api/personas/{id}/files/{filename} | &#x60;GET /api/personas/{id}/files/{filename}&#x60; — read a persona file slot.
[**getSkillAccess**](PersonasApi.md#getskillaccess) | **GET** /api/personas/{id}/skill-access | &#x60;GET /api/personas/{id}/skill-access&#x60; — get skill access config.
[**listPersonas**](PersonasApi.md#listpersonas) | **GET** /api/personas | &#x60;GET /api/personas&#x60; — list all personas defined on the server.
[**patchSkillAccessMode**](PersonasApi.md#patchskillaccessmode) | **PATCH** /api/personas/{id}/skill-access | &#x60;PATCH /api/personas/{id}/skill-access&#x60; — set skill access mode.
[**putPersonaFile**](PersonasApi.md#putpersonafile) | **PUT** /api/personas/{id}/files/{filename} | &#x60;PUT /api/personas/{id}/files/{filename}&#x60; — write a persona file slot.
[**setActivePersona**](PersonasApi.md#setactivepersona) | **POST** /api/personas/active | &#x60;POST /api/personas/active&#x60; — switch the active persona for the session.
[**updatePersona**](PersonasApi.md#updatepersona) | **PATCH** /api/personas/{id} | &#x60;PATCH /api/personas/{id}&#x60; — update a persona&#39;s starter prompts.


# **addSkillAccess**
> PersonaSkillAccess addSkillAccess(id, addSkillAccessRequest, xSpaceId)

`POST /api/personas/{id}/skill-access/skills` — add a skill to the access list.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final AddSkillAccessRequest addSkillAccessRequest = ; // AddSkillAccessRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.addSkillAccess(id, addSkillAccessRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->addSkillAccess: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Persona ID | 
 **addSkillAccessRequest** | [**AddSkillAccessRequest**](AddSkillAccessRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**PersonaSkillAccess**](PersonaSkillAccess.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createPersona**
> PersonaDetail createPersona(createPersonaRequest, xSpaceId)

`POST /api/personas` — create a new persona.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final CreatePersonaRequest createPersonaRequest = ; // CreatePersonaRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.createPersona(createPersonaRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->createPersona: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createPersonaRequest** | [**CreatePersonaRequest**](CreatePersonaRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**PersonaDetail**](PersonaDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteSkillAccess**
> deleteSkillAccess(id, skillName, xSpaceId)

`DELETE /api/personas/{id}/skill-access/skills/{skill_name}` — remove a skill from the access list.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final String skillName = skillName_example; // String | Skill name to remove
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    api.deleteSkillAccess(id, skillName, xSpaceId);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->deleteSkillAccess: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Persona ID | 
 **skillName** | **String**| Skill name to remove | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getPersona**
> PersonaDetail getPersona(id, xSpaceId)

`GET /api/personas/{id}` — get full persona detail.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.getPersona(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->getPersona: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Persona ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**PersonaDetail**](PersonaDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getPersonaFile**
> PersonaFileContent getPersonaFile(id, filename, xSpaceId)

`GET /api/personas/{id}/files/{filename}` — read a persona file slot.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final String filename = filename_example; // String | File slot name (e.g. SOUL.md)
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.getPersonaFile(id, filename, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->getPersonaFile: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Persona ID | 
 **filename** | **String**| File slot name (e.g. SOUL.md) | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**PersonaFileContent**](PersonaFileContent.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getSkillAccess**
> PersonaSkillAccess getSkillAccess(id, xSpaceId)

`GET /api/personas/{id}/skill-access` — get skill access config.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.getSkillAccess(id, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->getSkillAccess: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Persona ID | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**PersonaSkillAccess**](PersonaSkillAccess.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listPersonas**
> BuiltList<PersonaSummary> listPersonas(xSpaceId)

`GET /api/personas` — list all personas defined on the server.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.listPersonas(xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->listPersonas: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**BuiltList&lt;PersonaSummary&gt;**](PersonaSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **patchSkillAccessMode**
> PersonaSkillAccess patchSkillAccessMode(id, setSkillAccessModeRequest, xSpaceId)

`PATCH /api/personas/{id}/skill-access` — set skill access mode.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final SetSkillAccessModeRequest setSkillAccessModeRequest = ; // SetSkillAccessModeRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.patchSkillAccessMode(id, setSkillAccessModeRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->patchSkillAccessMode: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Persona ID | 
 **setSkillAccessModeRequest** | [**SetSkillAccessModeRequest**](SetSkillAccessModeRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**PersonaSkillAccess**](PersonaSkillAccess.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **putPersonaFile**
> PersonaFileContent putPersonaFile(id, filename, writePersonaFileRequest, xSpaceId)

`PUT /api/personas/{id}/files/{filename}` — write a persona file slot.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final String filename = filename_example; // String | File slot name (e.g. SOUL.md)
final WritePersonaFileRequest writePersonaFileRequest = ; // WritePersonaFileRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.putPersonaFile(id, filename, writePersonaFileRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->putPersonaFile: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Persona ID | 
 **filename** | **String**| File slot name (e.g. SOUL.md) | 
 **writePersonaFileRequest** | [**WritePersonaFileRequest**](WritePersonaFileRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**PersonaFileContent**](PersonaFileContent.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **setActivePersona**
> PersonaSummary setActivePersona(setActivePersonaRequest, xSpaceId)

`POST /api/personas/active` — switch the active persona for the session.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final SetActivePersonaRequest setActivePersonaRequest = ; // SetActivePersonaRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.setActivePersona(setActivePersonaRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->setActivePersona: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **setActivePersonaRequest** | [**SetActivePersonaRequest**](SetActivePersonaRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**PersonaSummary**](PersonaSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updatePersona**
> PersonaDetail updatePersona(id, updatePersonaRequest, xSpaceId)

`PATCH /api/personas/{id}` — update a persona's starter prompts.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final UpdatePersonaRequest updatePersonaRequest = ; // UpdatePersonaRequest | 
final String xSpaceId = xSpaceId_example; // String | Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter.

try {
    final response = api.updatePersona(id, updatePersonaRequest, xSpaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->updatePersona: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Persona ID | 
 **updatePersonaRequest** | [**UpdatePersonaRequest**](UpdatePersonaRequest.md)|  | 
 **xSpaceId** | **String**| Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when the caller holds a role in exactly one space. Clients that cannot set headers, such as the browser `EventSource`, may send the same value as the `space_id` query parameter. | [optional] 

### Return type

[**PersonaDetail**](PersonaDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

