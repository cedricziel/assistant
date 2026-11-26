# assistant_api.model.ToolPermissionRuleSummary

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**createdAt** | **String** | RFC 3339 creation time. | 
**effect** | **String** | `allow` or `deny`. | 
**id** | **String** |  | 
**rule** | **String** | Rule text, for example `bash(git status*)`. | 
**toolName** | **String** | The tool part of the rule, for example `bash`. | 
**userId** | **String** | The user the rule belongs to. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


