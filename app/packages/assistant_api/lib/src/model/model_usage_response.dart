//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'model_usage_response.g.dart';

/// Per-model token usage breakdown.
///
/// Properties:
/// * [avgDurationS] 
/// * [inputTokens] 
/// * [model] 
/// * [outputTokens] 
/// * [requestCount] 
@BuiltValue()
abstract class ModelUsageResponse implements Built<ModelUsageResponse, ModelUsageResponseBuilder> {
  @BuiltValueField(wireName: r'avg_duration_s')
  double get avgDurationS;

  @BuiltValueField(wireName: r'input_tokens')
  int get inputTokens;

  @BuiltValueField(wireName: r'model')
  String get model;

  @BuiltValueField(wireName: r'output_tokens')
  int get outputTokens;

  @BuiltValueField(wireName: r'request_count')
  int get requestCount;

  ModelUsageResponse._();

  factory ModelUsageResponse([void updates(ModelUsageResponseBuilder b)]) = _$ModelUsageResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ModelUsageResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ModelUsageResponse> get serializer => _$ModelUsageResponseSerializer();
}

class _$ModelUsageResponseSerializer implements PrimitiveSerializer<ModelUsageResponse> {
  @override
  final Iterable<Type> types = const [ModelUsageResponse, _$ModelUsageResponse];

  @override
  final String wireName = r'ModelUsageResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ModelUsageResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'avg_duration_s';
    yield serializers.serialize(
      object.avgDurationS,
      specifiedType: const FullType(double),
    );
    yield r'input_tokens';
    yield serializers.serialize(
      object.inputTokens,
      specifiedType: const FullType(int),
    );
    yield r'model';
    yield serializers.serialize(
      object.model,
      specifiedType: const FullType(String),
    );
    yield r'output_tokens';
    yield serializers.serialize(
      object.outputTokens,
      specifiedType: const FullType(int),
    );
    yield r'request_count';
    yield serializers.serialize(
      object.requestCount,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ModelUsageResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ModelUsageResponseBuilder result,
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
        case r'input_tokens':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.inputTokens = valueDes;
          break;
        case r'model':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.model = valueDes;
          break;
        case r'output_tokens':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.outputTokens = valueDes;
          break;
        case r'request_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.requestCount = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ModelUsageResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ModelUsageResponseBuilder();
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

