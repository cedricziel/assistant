# assistant_api.model.Artifact

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**artifactId** | **String** | Unique identifier (UUID) for the artifact, unique within a task. | 
**description** | **String** | A human-readable description of the artifact. | [optional] 
**extensions** | **BuiltList&lt;String&gt;** | The URIs of extensions present or contributing to this artifact. | [optional] 
**metadata** | [**BuiltMap&lt;String, JsonObject&gt;**](JsonObject.md) | Metadata included with the artifact. | [optional] 
**name** | **String** | A human-readable name for the artifact. | [optional] 
**parts** | [**BuiltList&lt;ModelPart&gt;**](ModelPart.md) | The content of the artifact. Must contain at least one part. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


