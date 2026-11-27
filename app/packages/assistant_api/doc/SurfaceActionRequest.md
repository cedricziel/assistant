# assistant_api.model.SurfaceActionRequest

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**context** | [**JsonObject**](.md) | The action's context payload. | 
**dataModel** | [**JsonObject**](.md) | The surface's data model at the time of the action. | [optional] 
**name** | **String** | The action name, which the surface must declare on a Button. | 
**sourceComponentId** | **String** | Id of the Button component that fired the action. | 
**surfaceId** | **String** | The surface the action belongs to. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


