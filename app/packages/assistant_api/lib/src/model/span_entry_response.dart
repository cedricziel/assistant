//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'span_entry_response.g.dart';

/// A single span within a trace.
///
/// Properties:
/// * [attributes] 
/// * [durationMs] 
/// * [name] 
/// * [spanId] 
/// * [startTime] 
@BuiltValue()
abstract class SpanEntryResponse implements Built<SpanEntryResponse, SpanEntryResponseBuilder> {
  @BuiltValueField(wireName: r'attributes')
  JsonObject? get attributes;

  @BuiltValueField(wireName: r'duration_ms')
  int get durationMs;

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'span_id')
  String get spanId;

  @BuiltValueField(wireName: r'start_time')
  DateTime get startTime;

  SpanEntryResponse._();

  factory SpanEntryResponse([void updates(SpanEntryResponseBuilder b)]) = _$SpanEntryResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SpanEntryResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SpanEntryResponse> get serializer => _$SpanEntryResponseSerializer();
}

class _$SpanEntryResponseSerializer implements PrimitiveSerializer<SpanEntryResponse> {
  @override
  final Iterable<Type> types = const [SpanEntryResponse, _$SpanEntryResponse];

  @override
  final String wireName = r'SpanEntryResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SpanEntryResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'attributes';
    yield object.attributes == null ? null : serializers.serialize(
      object.attributes,
      specifiedType: const FullType.nullable(JsonObject),
    );
    yield r'duration_ms';
    yield serializers.serialize(
      object.durationMs,
      specifiedType: const FullType(int),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'span_id';
    yield serializers.serialize(
      object.spanId,
      specifiedType: const FullType(String),
    );
    yield r'start_time';
    yield serializers.serialize(
      object.startTime,
      specifiedType: const FullType(DateTime),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SpanEntryResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SpanEntryResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'attributes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.attributes = valueDes;
          break;
        case r'duration_ms':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.durationMs = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'span_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.spanId = valueDes;
          break;
        case r'start_time':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.startTime = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SpanEntryResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SpanEntryResponseBuilder();
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

