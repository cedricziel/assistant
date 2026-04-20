//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/sse_thinking_event.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_subagent_thinking_event.g.dart';

/// SSE `subagent_thinking` event — a thinking token from a subagent.
///
/// Properties:
/// * [agentId] - The subagent that produced this thinking token.
/// * [data] - Inner event payload.
@BuiltValue()
abstract class SseSubagentThinkingEvent
    implements
        Built<SseSubagentThinkingEvent, SseSubagentThinkingEventBuilder> {
  /// The subagent that produced this thinking token.
  @BuiltValueField(wireName: r'agent_id')
  String get agentId;

  /// Inner event payload.
  @BuiltValueField(wireName: r'data')
  SseThinkingEvent get data;

  SseSubagentThinkingEvent._();

  factory SseSubagentThinkingEvent(
          [void updates(SseSubagentThinkingEventBuilder b)]) =
      _$SseSubagentThinkingEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseSubagentThinkingEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseSubagentThinkingEvent> get serializer =>
      _$SseSubagentThinkingEventSerializer();
}

class _$SseSubagentThinkingEventSerializer
    implements PrimitiveSerializer<SseSubagentThinkingEvent> {
  @override
  final Iterable<Type> types = const [
    SseSubagentThinkingEvent,
    _$SseSubagentThinkingEvent
  ];

  @override
  final String wireName = r'SseSubagentThinkingEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseSubagentThinkingEvent object, {
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
      specifiedType: const FullType(SseThinkingEvent),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SseSubagentThinkingEvent object, {
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
    required SseSubagentThinkingEventBuilder result,
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
            specifiedType: const FullType(SseThinkingEvent),
          ) as SseThinkingEvent;
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
  SseSubagentThinkingEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseSubagentThinkingEventBuilder();
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
