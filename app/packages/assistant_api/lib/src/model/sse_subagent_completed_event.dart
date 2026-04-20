//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_subagent_completed_event.g.dart';

/// SSE `subagent_completed` event — a subagent has finished.
///
/// Properties:
/// * [agentId] - Unique identifier of the subagent.
/// * [status] - Completion status (e.g. \"success\", \"error\").
/// * [summary] - Summary of the subagent's work.
@BuiltValue()
abstract class SseSubagentCompletedEvent
    implements
        Built<SseSubagentCompletedEvent, SseSubagentCompletedEventBuilder> {
  /// Unique identifier of the subagent.
  @BuiltValueField(wireName: r'agent_id')
  String get agentId;

  /// Completion status (e.g. \"success\", \"error\").
  @BuiltValueField(wireName: r'status')
  String get status;

  /// Summary of the subagent's work.
  @BuiltValueField(wireName: r'summary')
  String? get summary;

  SseSubagentCompletedEvent._();

  factory SseSubagentCompletedEvent(
          [void updates(SseSubagentCompletedEventBuilder b)]) =
      _$SseSubagentCompletedEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseSubagentCompletedEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseSubagentCompletedEvent> get serializer =>
      _$SseSubagentCompletedEventSerializer();
}

class _$SseSubagentCompletedEventSerializer
    implements PrimitiveSerializer<SseSubagentCompletedEvent> {
  @override
  final Iterable<Type> types = const [
    SseSubagentCompletedEvent,
    _$SseSubagentCompletedEvent
  ];

  @override
  final String wireName = r'SseSubagentCompletedEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseSubagentCompletedEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'agent_id';
    yield serializers.serialize(
      object.agentId,
      specifiedType: const FullType(String),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    if (object.summary != null) {
      yield r'summary';
      yield serializers.serialize(
        object.summary,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    SseSubagentCompletedEvent object, {
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
    required SseSubagentCompletedEventBuilder result,
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
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'summary':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.summary = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SseSubagentCompletedEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseSubagentCompletedEventBuilder();
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
