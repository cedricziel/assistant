# assistant_api.api.MembersApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**addMember**](MembersApi.md#addmember) | **POST** /api/orgs/{org_id}/spaces/{space_id}/members | &#x60;POST /api/orgs/{org_id}/spaces/{space_id}/members&#x60; — add a member.
[**listMembers**](MembersApi.md#listmembers) | **GET** /api/orgs/{org_id}/spaces/{space_id}/members | &#x60;GET /api/orgs/{org_id}/spaces/{space_id}/members&#x60; — list space members.
[**removeMember**](MembersApi.md#removemember) | **DELETE** /api/orgs/{org_id}/spaces/{space_id}/members/{user_id} | &#x60;DELETE /api/orgs/{org_id}/spaces/{space_id}/members/{user_id}&#x60; — remove member.
[**updateMember**](MembersApi.md#updatemember) | **PATCH** /api/orgs/{org_id}/spaces/{space_id}/members/{user_id} | &#x60;PATCH /api/orgs/{org_id}/spaces/{space_id}/members/{user_id}&#x60; — change role.


# **addMember**
> MemberEntry addMember(orgId, spaceId, addMemberRequest)

`POST /api/orgs/{org_id}/spaces/{space_id}/members` — add a member.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getMembersApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID
final AddMemberRequest addMemberRequest = ; // AddMemberRequest | 

try {
    final response = api.addMember(orgId, spaceId, addMemberRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling MembersApi->addMember: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 
 **addMemberRequest** | [**AddMemberRequest**](AddMemberRequest.md)|  | 

### Return type

[**MemberEntry**](MemberEntry.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listMembers**
> BuiltList<MemberEntry> listMembers(orgId, spaceId)

`GET /api/orgs/{org_id}/spaces/{space_id}/members` — list space members.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getMembersApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID

try {
    final response = api.listMembers(orgId, spaceId);
    print(response);
} on DioException catch (e) {
    print('Exception when calling MembersApi->listMembers: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 

### Return type

[**BuiltList&lt;MemberEntry&gt;**](MemberEntry.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **removeMember**
> removeMember(orgId, spaceId, userId)

`DELETE /api/orgs/{org_id}/spaces/{space_id}/members/{user_id}` — remove member.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getMembersApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID
final String userId = userId_example; // String | User ID

try {
    api.removeMember(orgId, spaceId, userId);
} on DioException catch (e) {
    print('Exception when calling MembersApi->removeMember: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 
 **userId** | **String**| User ID | 

### Return type

void (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateMember**
> MemberEntry updateMember(orgId, spaceId, userId, updateMemberRequest)

`PATCH /api/orgs/{org_id}/spaces/{space_id}/members/{user_id}` — change role.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getMembersApi();
final String orgId = orgId_example; // String | Organization ID
final String spaceId = spaceId_example; // String | Space ID
final String userId = userId_example; // String | User ID
final UpdateMemberRequest updateMemberRequest = ; // UpdateMemberRequest | 

try {
    final response = api.updateMember(orgId, spaceId, userId, updateMemberRequest);
    print(response);
} on DioException catch (e) {
    print('Exception when calling MembersApi->updateMember: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **orgId** | **String**| Organization ID | 
 **spaceId** | **String**| Space ID | 
 **userId** | **String**| User ID | 
 **updateMemberRequest** | [**UpdateMemberRequest**](UpdateMemberRequest.md)|  | 

### Return type

[**MemberEntry**](MemberEntry.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

