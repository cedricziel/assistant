//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_token_event.g.dart';

/// SSE `token` event — a text chunk from the assistant's response.
///
/// Properties:
/// * [content] - The text content of this token chunk.
@BuiltValue()
abstract class SseTokenEvent
    implements Built<SseTokenEvent, SseTokenEventBuilder> {
  /// The text content of this token chunk.
  @BuiltValueField(wireName: r'content')
  String get content;

  SseTokenEvent._();

  factory SseTokenEvent([void updates(SseTokenEventBuilder b)]) =
      _$SseTokenEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseTokenEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseTokenEvent> get serializer =>
      _$SseTokenEventSerializer();
}

class _$SseTokenEventSerializer implements PrimitiveSerializer<SseTokenEvent> {
  @override
  final Iterable<Type> types = const [SseTokenEvent, _$SseTokenEvent];

  @override
  final String wireName = r'SseTokenEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseTokenEvent object, {
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
    SseTokenEvent object, {
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
    required SseTokenEventBuilder result,
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
  SseTokenEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseTokenEventBuilder();
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
