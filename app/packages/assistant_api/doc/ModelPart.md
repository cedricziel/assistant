# assistant_api.model.ModelPart

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**data** | [**JsonObject**](.md) | Arbitrary structured data as a JSON value. | [optional] 
**filename** | **String** | An optional filename for the file. | [optional] 
**mediaType** | **String** | The MIME type of the part content. | [optional] 
**metadata** | [**BuiltMap&lt;String, JsonObject&gt;**](JsonObject.md) | Metadata associated with this part. | [optional] 
**raw** | **String** | Raw byte content of a file, base64-encoded in JSON. | [optional] 
**text** | **String** | The string content of a text part. | [optional] 
**url** | **String** | A URL pointing to the file's content. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


