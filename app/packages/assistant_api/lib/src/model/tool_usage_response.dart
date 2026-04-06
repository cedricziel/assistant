//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'tool_usage_response.g.dart';

/// Tool invocation statistics.
///
/// Properties:
/// * [avgDurationS] 
/// * [invocations] 
/// * [toolName] 
@BuiltValue()
abstract class ToolUsageResponse implements Built<ToolUsageResponse, ToolUsageResponseBuilder> {
  @BuiltValueField(wireName: r'avg_duration_s')
  double get avgDurationS;

  @BuiltValueField(wireName: r'invocations')
  int get invocations;

  @BuiltValueField(wireName: r'tool_name')
  String get toolName;

  ToolUsageResponse._();

  factory ToolUsageResponse([void updates(ToolUsageResponseBuilder b)]) = _$ToolUsageResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ToolUsageResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ToolUsageResponse> get serializer => _$ToolUsageResponseSerializer();
}

class _$ToolUsageResponseSerializer implements PrimitiveSerializer<ToolUsageResponse> {
  @override
  final Iterable<Type> types = const [ToolUsageResponse, _$ToolUsageResponse];

  @override
  final String wireName = r'ToolUsageResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ToolUsageResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'avg_duration_s';
    yield serializers.serialize(
      object.avgDurationS,
      specifiedType: const FullType(double),
    );
    yield r'invocations';
    yield serializers.serialize(
      object.invocations,
      specifiedType: const FullType(int),
    );
    yield r'tool_name';
    yield serializers.serialize(
      object.toolName,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ToolUsageResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ToolUsageResponseBuilder result,
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
        case r'invocations':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.invocations = valueDes;
          break;
        case r'tool_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.toolName = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ToolUsageResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ToolUsageResponseBuilder();
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

