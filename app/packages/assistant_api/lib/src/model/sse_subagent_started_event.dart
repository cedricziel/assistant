//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_subagent_started_event.g.dart';

/// SSE `subagent_started` event — a subagent has been spawned.
///
/// Properties:
/// * [agentId] - Unique identifier of the subagent.
/// * [task] - The task description given to the subagent.
@BuiltValue()
abstract class SseSubagentStartedEvent
    implements Built<SseSubagentStartedEvent, SseSubagentStartedEventBuilder> {
  /// Unique identifier of the subagent.
  @BuiltValueField(wireName: r'agent_id')
  String get agentId;

  /// The task description given to the subagent.
  @BuiltValueField(wireName: r'task')
  String get task;

  SseSubagentStartedEvent._();

  factory SseSubagentStartedEvent(
          [void updates(SseSubagentStartedEventBuilder b)]) =
      _$SseSubagentStartedEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseSubagentStartedEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseSubagentStartedEvent> get serializer =>
      _$SseSubagentStartedEventSerializer();
}

class _$SseSubagentStartedEventSerializer
    implements PrimitiveSerializer<SseSubagentStartedEvent> {
  @override
  final Iterable<Type> types = const [
    SseSubagentStartedEvent,
    _$SseSubagentStartedEvent
  ];

  @override
  final String wireName = r'SseSubagentStartedEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseSubagentStartedEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'agent_id';
    yield serializers.serialize(
      object.agentId,
      specifiedType: const FullType(String),
    );
    yield r'task';
    yield serializers.serialize(
      object.task,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SseSubagentStartedEvent object, {
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
    required SseSubagentStartedEventBuilder result,
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
        case r'task':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.task = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SseSubagentStartedEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseSubagentStartedEventBuilder();
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
