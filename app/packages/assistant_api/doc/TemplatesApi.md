# assistant_api.api.TemplatesApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createFromTemplate**](TemplatesApi.md#createfromtemplate) | **POST** /api/orgs/{org_id}/spaces/{space_id}/personas/from-template | &#x60;POST /api/orgs/{org_id}/spaces/{space_id}/personas/from-template&#x60; — create a persona from a template.
[**listTemplates**](TemplatesApi.md#listtemplates) | **GET** /api/orgs/{org_id}/catalog/templates | &#x60;GET /api/orgs/{org_id}/catalog/templates&#x60; — list available persona templates.
[**onboardingStatus**](TemplatesApi.md#onboardingstatus) | **GET** /api/users/me/onboarding-status | &#x60;GET /api/users/me/onboarding-status&#x60; — check if user has created at least one persona.


# **createFromTemplate**
> PersonaFromTemplateResponse createFromTemplate(orgId, spaceId, createFromTemplateRequest)

`POST /api/orgs/{org_id}/spaces/{space_id}/personas/from-template` — create a persona from a template.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getTemplatesApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID
final CreateFromTemplateRequest createFromTemplateRequest = ; // CreateFromTemplateRequest | 

try {
    final response = api.createFromTemplate(orgId, spaceId, createFromTemplateRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling TemplatesApi->createFromTemplate: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 
 **createFromTemplateRequest** | [**CreateFromTemplateRequest**](CreateFromTemplateRequest.md)|  | 

### Return type

[**PersonaFromTemplateResponse**](PersonaFromTemplateResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listTemplates**
> BuiltList<TemplateResponse> listTemplates(orgId)

`GET /api/orgs/{org_id}/catalog/templates` — list available persona templates.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getTemplatesApi();
final String orgId = orgId_example; // String | Organization ID

try {
    final response = api.listTemplates(orgId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling TemplatesApi->listTemplates: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 

### Return type

[**BuiltList&lt;TemplateResponse&gt;**](TemplateResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **onboardingStatus**
> OnboardingStatusResponse onboardingStatus()

`GET /api/users/me/onboarding-status` — check if user has created at least one persona.

### Example
```dart
import 'package:assistant_api/api.dart';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';
// TODO Configure OAuth2 access token for authorization: oauth2
//defaultApiClient.getAuthentication<OAuth>('oauth2').accessToken = 'YOUR_ACCESS_TOKEN';

final api = AssistantApi().getTemplatesApi();

try {
    final response = api.onboardingStatus();
    print(response);
} on DioException catch (e) {
    print('Exception when calling TemplatesApi->onboardingStatus: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**OnboardingStatusResponse**](OnboardingStatusResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token), [oauth2](../README.md#oauth2), [oauth2](../README.md#oauth2)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

