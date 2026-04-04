# assistant_api.model.AgentCard

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**capabilities** | [**AgentCapabilities**](AgentCapabilities.md) | A2A capability set supported by the agent. | 
**defaultInputModes** | **BuiltList&lt;String&gt;** | Input modes the agent supports across all skills (media types). | 
**defaultOutputModes** | **BuiltList&lt;String&gt;** | Output media types supported by this agent. | 
**description** | **String** | A human-readable description of the agent's purpose. | 
**documentationUrl** | **String** | A URL providing additional documentation about the agent. | [optional] 
**iconUrl** | **String** | A URL to an icon for the agent. | [optional] 
**name** | **String** | A human-readable name for the agent. | 
**provider** | [**AgentProvider**](AgentProvider.md) | The service provider of the agent. | [optional] 
**securityRequirements** | [**BuiltList&lt;SecurityRequirement&gt;**](SecurityRequirement.md) | Security requirements for contacting the agent. | [optional] 
**securitySchemes** | [**BuiltMap&lt;String, SecurityScheme&gt;**](SecurityScheme.md) | Security scheme definitions for authenticating with this agent. | [optional] 
**signatures** | [**BuiltList&lt;AgentCardSignature&gt;**](AgentCardSignature.md) | JSON Web Signatures computed for this `AgentCard`. | [optional] 
**skills** | [**BuiltList&lt;AgentSkill&gt;**](AgentSkill.md) | Skills represent the abilities of an agent. | 
**supportedInterfaces** | [**BuiltList&lt;AgentInterface&gt;**](AgentInterface.md) | Ordered list of supported interfaces. The first entry is preferred. | 
**version** | **String** | The version of the agent (e.g., \"1.0.0\"). | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


