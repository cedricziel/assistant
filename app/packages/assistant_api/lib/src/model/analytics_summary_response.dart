//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/tool_usage_response.dart';
import 'package:assistant_api/src/model/time_series_response.dart';
import 'package:assistant_api/src/model/model_usage_response.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'analytics_summary_response.g.dart';

/// Aggregated analytics summary for a time window.
///
/// Properties:
/// * [avgDurationS]
/// * [errorCount]
/// * [models]
/// * [requestSeries]
/// * [tokenSeries]
/// * [tools]
/// * [totalRequests]
/// * [totalTokensIn]
/// * [totalTokensOut]
/// * [totalToolInvocations]
/// * [uniqueModels]
/// * [windowHours] - The requested window in hours.
@BuiltValue()
abstract class AnalyticsSummaryResponse
    implements
        Built<AnalyticsSummaryResponse, AnalyticsSummaryResponseBuilder> {
  @BuiltValueField(wireName: r'avg_duration_s')
  double get avgDurationS;

  @BuiltValueField(wireName: r'error_count')
  int get errorCount;

  @BuiltValueField(wireName: r'models')
  BuiltList<ModelUsageResponse> get models;

  @BuiltValueField(wireName: r'request_series')
  BuiltList<TimeSeriesResponse> get requestSeries;

  @BuiltValueField(wireName: r'token_series')
  BuiltList<TimeSeriesResponse> get tokenSeries;

  @BuiltValueField(wireName: r'tools')
  BuiltList<ToolUsageResponse> get tools;

  @BuiltValueField(wireName: r'total_requests')
  int get totalRequests;

  @BuiltValueField(wireName: r'total_tokens_in')
  int get totalTokensIn;

  @BuiltValueField(wireName: r'total_tokens_out')
  int get totalTokensOut;

  @BuiltValueField(wireName: r'total_tool_invocations')
  int get totalToolInvocations;

  @BuiltValueField(wireName: r'unique_models')
  BuiltList<String> get uniqueModels;

  /// The requested window in hours.
  @BuiltValueField(wireName: r'window_hours')
  int get windowHours;

  AnalyticsSummaryResponse._();

  factory AnalyticsSummaryResponse(
          [void updates(AnalyticsSummaryResponseBuilder b)]) =
      _$AnalyticsSummaryResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AnalyticsSummaryResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AnalyticsSummaryResponse> get serializer =>
      _$AnalyticsSummaryResponseSerializer();
}

class _$AnalyticsSummaryResponseSerializer
    implements PrimitiveSerializer<AnalyticsSummaryResponse> {
  @override
  final Iterable<Type> types = const [
    AnalyticsSummaryResponse,
    _$AnalyticsSummaryResponse
  ];

  @override
  final String wireName = r'AnalyticsSummaryResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AnalyticsSummaryResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'avg_duration_s';
    yield serializers.serialize(
      object.avgDurationS,
      specifiedType: const FullType(double),
    );
    yield r'error_count';
    yield serializers.serialize(
      object.errorCount,
      specifiedType: const FullType(int),
    );
    yield r'models';
    yield serializers.serialize(
      object.models,
      specifiedType: const FullType(BuiltList, [FullType(ModelUsageResponse)]),
    );
    yield r'request_series';
    yield serializers.serialize(
      object.requestSeries,
      specifiedType: const FullType(BuiltList, [FullType(TimeSeriesResponse)]),
    );
    yield r'token_series';
    yield serializers.serialize(
      object.tokenSeries,
      specifiedType: const FullType(BuiltList, [FullType(TimeSeriesResponse)]),
    );
    yield r'tools';
    yield serializers.serialize(
      object.tools,
      specifiedType: const FullType(BuiltList, [FullType(ToolUsageResponse)]),
    );
    yield r'total_requests';
    yield serializers.serialize(
      object.totalRequests,
      specifiedType: const FullType(int),
    );
    yield r'total_tokens_in';
    yield serializers.serialize(
      object.totalTokensIn,
      specifiedType: const FullType(int),
    );
    yield r'total_tokens_out';
    yield serializers.serialize(
      object.totalTokensOut,
      specifiedType: const FullType(int),
    );
    yield r'total_tool_invocations';
    yield serializers.serialize(
      object.totalToolInvocations,
      specifiedType: const FullType(int),
    );
    yield r'unique_models';
    yield serializers.serialize(
      object.uniqueModels,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'window_hours';
    yield serializers.serialize(
      object.windowHours,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AnalyticsSummaryResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object,
            specifiedType: specifiedType)
        .toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AnalyticsSummaryResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'avg_duration_s':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(double),
          ) as double;
          result.avgDurationS = valueDes;
          break;
        case r'error_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.errorCount = valueDes;
          break;
        case r'models':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(BuiltList, [FullType(ModelUsageResponse)]),
          ) as BuiltList<ModelUsageResponse>;
          result.models.replace(valueDes);
          break;
        case r'request_series':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(BuiltList, [FullType(TimeSeriesResponse)]),
          ) as BuiltList<TimeSeriesResponse>;
          result.requestSeries.replace(valueDes);
          break;
        case r'token_series':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(BuiltList, [FullType(TimeSeriesResponse)]),
          ) as BuiltList<TimeSeriesResponse>;
          result.tokenSeries.replace(valueDes);
          break;
        case r'tools':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(BuiltList, [FullType(ToolUsageResponse)]),
          ) as BuiltList<ToolUsageResponse>;
          result.tools.replace(valueDes);
          break;
        case r'total_requests':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.totalRequests = valueDes;
          break;
        case r'total_tokens_in':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.totalTokensIn = valueDes;
          break;
        case r'total_tokens_out':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.totalTokensOut = valueDes;
          break;
        case r'total_tool_invocations':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.totalToolInvocations = valueDes;
          break;
        case r'unique_models':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.uniqueModels.replace(valueDes);
          break;
        case r'window_hours':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.windowHours = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AnalyticsSummaryResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AnalyticsSummaryResponseBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
