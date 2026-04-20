//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_status_event.g.dart';

/// SSE `status` event — a status message (e.g. \"Calling tool X\").
///
/// Properties:
/// * [message] - Human-readable status message.
@BuiltValue()
abstract class SseStatusEvent
    implements Built<SseStatusEvent, SseStatusEventBuilder> {
  /// Human-readable status message.
  @BuiltValueField(wireName: r'message')
  String get message;

  SseStatusEvent._();

  factory SseStatusEvent([void updates(SseStatusEventBuilder b)]) =
      _$SseStatusEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseStatusEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseStatusEvent> get serializer =>
      _$SseStatusEventSerializer();
}

class _$SseStatusEventSerializer
    implements PrimitiveSerializer<SseStatusEvent> {
  @override
  final Iterable<Type> types = const [SseStatusEvent, _$SseStatusEvent];

  @override
  final String wireName = r'SseStatusEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseStatusEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'message';
    yield serializers.serialize(
      object.message,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SseStatusEvent object, {
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
    required SseStatusEventBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.message = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SseStatusEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseStatusEventBuilder();
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
