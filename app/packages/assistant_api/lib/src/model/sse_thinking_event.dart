//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_thinking_event.g.dart';

/// SSE `thinking` event — a thinking/reasoning token from the model.
///
/// Properties:
/// * [content] - The thinking text content.
@BuiltValue()
abstract class SseThinkingEvent
    implements Built<SseThinkingEvent, SseThinkingEventBuilder> {
  /// The thinking text content.
  @BuiltValueField(wireName: r'content')
  String get content;

  SseThinkingEvent._();

  factory SseThinkingEvent([void updates(SseThinkingEventBuilder b)]) =
      _$SseThinkingEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseThinkingEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseThinkingEvent> get serializer =>
      _$SseThinkingEventSerializer();
}

class _$SseThinkingEventSerializer
    implements PrimitiveSerializer<SseThinkingEvent> {
  @override
  final Iterable<Type> types = const [SseThinkingEvent, _$SseThinkingEvent];

  @override
  final String wireName = r'SseThinkingEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseThinkingEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'content';
    yield serializers.serialize(
      object.content,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SseThinkingEvent object, {
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
    required SseThinkingEventBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'content':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.content = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SseThinkingEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseThinkingEventBuilder();
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
