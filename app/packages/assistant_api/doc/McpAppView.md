# assistant_api.model.McpAppView

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**csp** | [**McpAppCspView**](McpAppCspView.md) |  | 
**html** | **String** | The app document, served as `text/html;profile=mcp-app`. | 
**permissions** | [**McpAppPermissionsView**](McpAppPermissionsView.md) |  | 
**prefersBorder** | **bool** | Whether the app asks to be drawn with a border. | [optional] 
**resourceUri** | **String** | The `ui://` resource the HTML was read from. | 
**server** | **String** | Namespace of the server the tool came from, as in `mcp__{server}__{tool}`. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


