# assistant_api.model.AnalyticsSummaryResponse

## Load the model package
```dart
import 'package:assistant_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**avgDurationS** | **double** |  | 
**errorCount** | **int** |  | 
**models** | [**BuiltList&lt;ModelUsageResponse&gt;**](ModelUsageResponse.md) |  | 
**requestSeries** | [**BuiltList&lt;TimeSeriesResponse&gt;**](TimeSeriesResponse.md) |  | 
**tokenSeries** | [**BuiltList&lt;TimeSeriesResponse&gt;**](TimeSeriesResponse.md) |  | 
**tools** | [**BuiltList&lt;ToolUsageResponse&gt;**](ToolUsageResponse.md) |  | 
**totalRequests** | **int** |  | 
**totalTokensIn** | **int** |  | 
**totalTokensOut** | **int** |  | 
**totalToolInvocations** | **int** |  | 
**uniqueModels** | **BuiltList&lt;String&gt;** |  | 
**windowHours** | **int** | The requested window in hours. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


