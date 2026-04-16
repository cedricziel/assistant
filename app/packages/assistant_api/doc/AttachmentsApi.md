# assistant_api.api.AttachmentsApi

## Load the API package
```dart
import 'package:assistant_api/api.dart';
```

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**serveAttachment**](AttachmentsApi.md#serveattachment) | **GET** /api/attachments/{id} | &#x60;GET /api/attachments/{id}&#x60; — serve an attachment, optionally resized.
[**uploadAttachment**](AttachmentsApi.md#uploadattachment) | **POST** /api/conversations/{id}/attachments | &#x60;POST /api/conversations/{id}/attachments&#x60; — upload an image attachment.


# **serveAttachment**
> serveAttachment(id, w, h)

`GET /api/attachments/{id}` — serve an attachment, optionally resized.

Supports `w` and `h` query params for on-demand image resizing. Resized variants are cached on disk. Responds with `ETag` and `Cache-Control` headers; returns `304 Not Modified` when `If-None-Match` matches.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getAttachmentsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Attachment ID
final int w = 56; // int | Desired width in pixels (preserves aspect ratio).
final int h = 56; // int | Desired height in pixels (preserves aspect ratio).

try {
    api.serveAttachment(id, w, h);
} on DioException catch (e) {
    print('Exception when calling AttachmentsApi->serveAttachment: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Attachment ID | 
 **w** | **int**| Desired width in pixels (preserves aspect ratio). | [optional] 
 **h** | **int**| Desired height in pixels (preserves aspect ratio). | [optional] 

### Return type

void (empty response body)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/octet-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **uploadAttachment**
> AttachmentMetaResponse uploadAttachment(id, file)

`POST /api/conversations/{id}/attachments` — upload an image attachment.

Accepts a `multipart/form-data` body with a single `file` field. Returns `201 Created` with the attachment metadata on success.

### Example
```dart
import 'package:assistant_api/api.dart';

final api = AssistantApi().getAttachmentsApi();
final String id = 38400000-8cf0-11bd-b23e-10b96e4ef00d; // String | Conversation ID
final MultipartFile file = BINARY_DATA_HERE; // MultipartFile | The image file.

try {
    final response = api.uploadAttachment(id, file);
    print(response);
} on DioException catch (e) {
    print('Exception when calling AttachmentsApi->uploadAttachment: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **String**| Conversation ID | 
 **file** | **MultipartFile**| The image file. | 

### Return type

[**AttachmentMetaResponse**](AttachmentMetaResponse.md)

### Authorization

[bearer_token](../README.md#bearer_token)

### HTTP request headers

 - **Content-Type**: multipart/form-data
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

