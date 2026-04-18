//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'log_entry_response.g.dart';

/// A log entry row returned by the API.
///
/// Properties:
/// * [conversationId]
/// * [fields]
/// * [id]
/// * [message]
/// * [severity]
/// * [target]
/// * [timestamp]
/// * [traceId]
@BuiltValue()
abstract class LogEntryResponse
    implements Built<LogEntryResponse, LogEntryResponseBuilder> {
  @BuiltValueField(wireName: r'conversation_id')
  String? get conversationId;

  @BuiltValueField(wireName: r'fields')
  JsonObject? get fields;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'message')
  String get message;

  @BuiltValueField(wireName: r'severity')
  String get severity;

  @BuiltValueField(wireName: r'target')
  String get target;

  @BuiltValueField(wireName: r'timestamp')
  DateTime get timestamp;

  @BuiltValueField(wireName: r'trace_id')
  String? get traceId;

  LogEntryResponse._();

  factory LogEntryResponse([void updates(LogEntryResponseBuilder b)]) =
      _$LogEntryResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(LogEntryResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<LogEntryResponse> get serializer =>
      _$LogEntryResponseSerializer();
}

class _$LogEntryResponseSerializer
    implements PrimitiveSerializer<LogEntryResponse> {
  @override
  final Iterable<Type> types = const [LogEntryResponse, _$LogEntryResponse];

  @override
  final String wireName = r'LogEntryResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    LogEntryResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.conversationId != null) {
      yield r'conversation_id';
      yield serializers.serialize(
        object.conversationId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'fields';
    yield object.fields == null
        ? null
        : serializers.serialize(
            object.fields,
            specifiedType: const FullType.nullable(JsonObject),
          );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'message';
    yield serializers.serialize(
      object.message,
      specifiedType: const FullType(String),
    );
    yield r'severity';
    yield serializers.serialize(
      object.severity,
      specifiedType: const FullType(String),
    );
    yield r'target';
    yield serializers.serialize(
      object.target,
      specifiedType: const FullType(String),
    );
    yield r'timestamp';
    yield serializers.serialize(
      object.timestamp,
      specifiedType: const FullType(DateTime),
    );
    if (object.traceId != null) {
      yield r'trace_id';
      yield serializers.serialize(
        object.traceId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    LogEntryResponse object, {
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
    required LogEntryResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'conversation_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.conversationId = valueDes;
          break;
        case r'fields':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.fields = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.message = valueDes;
          break;
        case r'severity':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.severity = valueDes;
          break;
        case r'target':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.target = valueDes;
          break;
        case r'timestamp':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.timestamp = valueDes;
          break;
        case r'trace_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.traceId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  LogEntryResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = LogEntryResponseBuilder();
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
