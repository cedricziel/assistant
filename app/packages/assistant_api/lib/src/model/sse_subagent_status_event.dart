//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/sse_status_event.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_subagent_status_event.g.dart';

/// SSE `subagent_status` event — a status update from a subagent.
///
/// Properties:
/// * [agentId] - The subagent that emitted this status.
/// * [data] - Inner event payload.
@BuiltValue()
abstract class SseSubagentStatusEvent
    implements Built<SseSubagentStatusEvent, SseSubagentStatusEventBuilder> {
  /// The subagent that emitted this status.
  @BuiltValueField(wireName: r'agent_id')
  String get agentId;

  /// Inner event payload.
  @BuiltValueField(wireName: r'data')
  SseStatusEvent get data;

  SseSubagentStatusEvent._();

  factory SseSubagentStatusEvent(
          [void updates(SseSubagentStatusEventBuilder b)]) =
      _$SseSubagentStatusEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseSubagentStatusEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseSubagentStatusEvent> get serializer =>
      _$SseSubagentStatusEventSerializer();
}

class _$SseSubagentStatusEventSerializer
    implements PrimitiveSerializer<SseSubagentStatusEvent> {
  @override
  final Iterable<Type> types = const [
    SseSubagentStatusEvent,
    _$SseSubagentStatusEvent
  ];

  @override
  final String wireName = r'SseSubagentStatusEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseSubagentStatusEvent object, {
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
      specifiedType: const FullType(SseStatusEvent),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SseSubagentStatusEvent object, {
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
    required SseSubagentStatusEventBuilder result,
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
            specifiedType: const FullType(SseStatusEvent),
          ) as SseStatusEvent;
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
  SseSubagentStatusEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseSubagentStatusEventBuilder();
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
