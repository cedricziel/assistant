# assistant_api.api.SkillsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**listPersonaSkills**](SkillsApi.md#listpersonaskills) | **GET** /api/personas/{persona_id}/skills | &#x60;GET /api/personas/{persona_id}/skills&#x60; — list skills for a persona.


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

