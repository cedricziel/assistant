# assistant_api.model.AgentSkill

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**description** | **String** | A detailed description of the skill. | 
**examples** | **BuiltList&lt;String&gt;** | Example prompts or scenarios that this skill can handle. | [optional] 
**id** | **String** | A unique identifier for the skill. | 
**inputModes** | **BuiltList&lt;String&gt;** | Supported input media types, overriding agent defaults. | [optional] 
**name** | **String** | A human-readable name for the skill. | 
**outputModes** | **BuiltList&lt;String&gt;** | Supported output media types, overriding agent defaults. | [optional] 
**securityRequirements** | [**BuiltList&lt;SecurityRequirement&gt;**](SecurityRequirement.md) | Security schemes necessary for this skill. | [optional] 
**tags** | **BuiltList&lt;String&gt;** | Keywords describing the skill's capabilities. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


