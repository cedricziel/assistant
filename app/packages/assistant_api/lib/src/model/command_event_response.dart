//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'command_event_response.g.dart';

/// Response body for a command event.
///
/// Properties:
/// * [ackText] - Acknowledgement text returned by the command.
/// * [command] - Command that was executed.
/// * [createdAt] - When the event was created.
/// * [eventType] - Event type (always `\"command\"` for now).
/// * [id] - Unique event ID.
/// * [payload]
@BuiltValue()
abstract class CommandEventResponse
    implements Built<CommandEventResponse, CommandEventResponseBuilder> {
  /// Acknowledgement text returned by the command.
  @BuiltValueField(wireName: r'ack_text')
  String? get ackText;

  /// Command that was executed.
  @BuiltValueField(wireName: r'command')
  String get command;

  /// When the event was created.
  @BuiltValueField(wireName: r'created_at')
  DateTime get createdAt;

  /// Event type (always `\"command\"` for now).
  @BuiltValueField(wireName: r'event_type')
  String get eventType;

  /// Unique event ID.
  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'payload')
  JsonObject? get payload;

  CommandEventResponse._();

  factory CommandEventResponse([void updates(CommandEventResponseBuilder b)]) =
      _$CommandEventResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CommandEventResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CommandEventResponse> get serializer =>
      _$CommandEventResponseSerializer();
}

class _$CommandEventResponseSerializer
    implements PrimitiveSerializer<CommandEventResponse> {
  @override
  final Iterable<Type> types = const [
    CommandEventResponse,
    _$CommandEventResponse
  ];

  @override
  final String wireName = r'CommandEventResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CommandEventResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.ackText != null) {
      yield r'ack_text';
      yield serializers.serialize(
        object.ackText,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'command';
    yield serializers.serialize(
      object.command,
      specifiedType: const FullType(String),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'event_type';
    yield serializers.serialize(
      object.eventType,
      specifiedType: const FullType(String),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    if (object.payload != null) {
      yield r'payload';
      yield serializers.serialize(
        object.payload,
        specifiedType: const FullType.nullable(JsonObject),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    CommandEventResponse object, {
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
    required CommandEventResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'ack_text':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.ackText = valueDes;
          break;
        case r'command':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.command = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.createdAt = valueDes;
          break;
        case r'event_type':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.eventType = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'payload':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.payload = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CommandEventResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CommandEventResponseBuilder();
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
