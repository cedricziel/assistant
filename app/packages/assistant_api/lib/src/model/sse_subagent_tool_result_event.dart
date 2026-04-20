//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/sse_tool_result_event.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_subagent_tool_result_event.g.dart';

/// SSE `subagent_tool_result` event — a tool result from a subagent.
///
/// Properties:
/// * [agentId] - The subagent that executed the tool.
/// * [data] - Inner event payload.
@BuiltValue()
abstract class SseSubagentToolResultEvent
    implements
        Built<SseSubagentToolResultEvent, SseSubagentToolResultEventBuilder> {
  /// The subagent that executed the tool.
  @BuiltValueField(wireName: r'agent_id')
  String get agentId;

  /// Inner event payload.
  @BuiltValueField(wireName: r'data')
  SseToolResultEvent get data;

  SseSubagentToolResultEvent._();

  factory SseSubagentToolResultEvent(
          [void updates(SseSubagentToolResultEventBuilder b)]) =
      _$SseSubagentToolResultEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseSubagentToolResultEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseSubagentToolResultEvent> get serializer =>
      _$SseSubagentToolResultEventSerializer();
}

class _$SseSubagentToolResultEventSerializer
    implements PrimitiveSerializer<SseSubagentToolResultEvent> {
  @override
  final Iterable<Type> types = const [
    SseSubagentToolResultEvent,
    _$SseSubagentToolResultEvent
  ];

  @override
  final String wireName = r'SseSubagentToolResultEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseSubagentToolResultEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'agent_id';
    yield serializers.serialize(
      object.agentId,
      specifiedType: const FullType(String),
    );
    yield r'data';
    yield serializers.serialize(
      object.data,
      specifiedType: const FullType(SseToolResultEvent),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SseSubagentToolResultEvent object, {
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
    required SseSubagentToolResultEventBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'agent_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.agentId = valueDes;
          break;
        case r'data':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(SseToolResultEvent),
          ) as SseToolResultEvent;
          result.data.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SseSubagentToolResultEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseSubagentToolResultEventBuilder();
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
