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


# **addSkillAccess**
> PersonaSkillAccess addSkillAccess(id, addSkillAccessRequest)

`POST /api/personas/{id}/skill-access/skills` — add a skill to the access list.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final AddSkillAccessRequest addSkillAccessRequest = ; // AddSkillAccessRequest | 

try {
    final response = api.addSkillAccess(id, addSkillAccessRequest);
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

### Return type

[**PersonaSkillAccess**](PersonaSkillAccess.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createPersona**
> PersonaDetail createPersona(createPersonaRequest)

`POST /api/personas` — create a new persona.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final CreatePersonaRequest createPersonaRequest = ; // CreatePersonaRequest | 

try {
    final response = api.createPersona(createPersonaRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->createPersona: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createPersonaRequest** | [**CreatePersonaRequest**](CreatePersonaRequest.md)|  | 

### Return type

[**PersonaDetail**](PersonaDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteSkillAccess**
> deleteSkillAccess(id, skillName)

`DELETE /api/personas/{id}/skill-access/skills/{skill_name}` — remove a skill from the access list.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final String skillName = skillName_example; // String | Skill name to remove

try {
    api.deleteSkillAccess(id, skillName);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->deleteSkillAccess: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Persona ID | 
 **skillName** | **String**| Skill name to remove | 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getPersona**
> PersonaDetail getPersona(id)

`GET /api/personas/{id}` — get full persona detail.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID

try {
    final response = api.getPersona(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->getPersona: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Persona ID | 

### Return type

[**PersonaDetail**](PersonaDetail.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getPersonaFile**
> PersonaFileContent getPersonaFile(id, filename)

`GET /api/personas/{id}/files/{filename}` — read a persona file slot.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final String filename = filename_example; // String | File slot name (e.g. SOUL.md)

try {
    final response = api.getPersonaFile(id, filename);
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

### Return type

[**PersonaFileContent**](PersonaFileContent.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getSkillAccess**
> PersonaSkillAccess getSkillAccess(id)

`GET /api/personas/{id}/skill-access` — get skill access config.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID

try {
    final response = api.getSkillAccess(id);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->getSkillAccess: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Persona ID | 

### Return type

[**PersonaSkillAccess**](PersonaSkillAccess.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listPersonas**
> BuiltList<PersonaSummary> listPersonas()

`GET /api/personas` — list all personas defined on the server.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();

try {
    final response = api.listPersonas();
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->listPersonas: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**BuiltList&lt;PersonaSummary&gt;**](PersonaSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **patchSkillAccessMode**
> PersonaSkillAccess patchSkillAccessMode(id, setSkillAccessModeRequest)

`PATCH /api/personas/{id}/skill-access` — set skill access mode.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final SetSkillAccessModeRequest setSkillAccessModeRequest = ; // SetSkillAccessModeRequest | 

try {
    final response = api.patchSkillAccessMode(id, setSkillAccessModeRequest);
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

### Return type

[**PersonaSkillAccess**](PersonaSkillAccess.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **putPersonaFile**
> PersonaFileContent putPersonaFile(id, filename, writePersonaFileRequest)

`PUT /api/personas/{id}/files/{filename}` — write a persona file slot.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final String id = id_example; // String | Persona ID
final String filename = filename_example; // String | File slot name (e.g. SOUL.md)
final WritePersonaFileRequest writePersonaFileRequest = ; // WritePersonaFileRequest | 

try {
    final response = api.putPersonaFile(id, filename, writePersonaFileRequest);
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

### Return type

[**PersonaFileContent**](PersonaFileContent.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **setActivePersona**
> PersonaSummary setActivePersona(setActivePersonaRequest)

`POST /api/personas/active` — switch the active persona for the session.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getPersonasApi();
final SetActivePersonaRequest setActivePersonaRequest = ; // SetActivePersonaRequest | 

try {
    final response = api.setActivePersona(setActivePersonaRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling PersonasApi->setActivePersona: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **setActivePersonaRequest** | [**SetActivePersonaRequest**](SetActivePersonaRequest.md)|  | 

### Return type

[**PersonaSummary**](PersonaSummary.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

