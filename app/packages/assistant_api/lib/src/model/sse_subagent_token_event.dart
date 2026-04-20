//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/sse_token_event.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_subagent_token_event.g.dart';

/// SSE `subagent_token` event — a text token from a subagent's response.
///
/// Properties:
/// * [agentId] - The subagent that produced this token.
/// * [data] - Inner event payload.
@BuiltValue()
abstract class SseSubagentTokenEvent
    implements Built<SseSubagentTokenEvent, SseSubagentTokenEventBuilder> {
  /// The subagent that produced this token.
  @BuiltValueField(wireName: r'agent_id')
  String get agentId;

  /// Inner event payload.
  @BuiltValueField(wireName: r'data')
  SseTokenEvent get data;

  SseSubagentTokenEvent._();

  factory SseSubagentTokenEvent(
      [void updates(SseSubagentTokenEventBuilder b)]) = _$SseSubagentTokenEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseSubagentTokenEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseSubagentTokenEvent> get serializer =>
      _$SseSubagentTokenEventSerializer();
}

class _$SseSubagentTokenEventSerializer
    implements PrimitiveSerializer<SseSubagentTokenEvent> {
  @override
  final Iterable<Type> types = const [
    SseSubagentTokenEvent,
    _$SseSubagentTokenEvent
  ];

  @override
  final String wireName = r'SseSubagentTokenEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseSubagentTokenEvent object, {
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
      specifiedType: const FullType(SseTokenEvent),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SseSubagentTokenEvent object, {
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
    required SseSubagentTokenEventBuilder result,
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
            specifiedType: const FullType(SseTokenEvent),
          ) as SseTokenEvent;
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
  SseSubagentTokenEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseSubagentTokenEventBuilder();
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
