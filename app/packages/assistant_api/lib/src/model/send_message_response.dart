//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/message.dart';
import 'package:assistant_api/src/model/task.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'send_message_response.g.dart';

/// Response for the `SendMessage` method.  Contains either a task or a message (oneof payload in the proto).
///
/// Properties:
/// * [message] - A message from the agent.
/// * [task] - The task created or updated by the message.
@BuiltValue()
abstract class SendMessageResponse
    implements Built<SendMessageResponse, SendMessageResponseBuilder> {
  /// A message from the agent.
  @BuiltValueField(wireName: r'message')
  Message? get message;

  /// The task created or updated by the message.
  @BuiltValueField(wireName: r'task')
  Task? get task;

  SendMessageResponse._();

  factory SendMessageResponse([void updates(SendMessageResponseBuilder b)]) =
      _$SendMessageResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SendMessageResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SendMessageResponse> get serializer =>
      _$SendMessageResponseSerializer();
}

class _$SendMessageResponseSerializer
    implements PrimitiveSerializer<SendMessageResponse> {
  @override
  final Iterable<Type> types = const [
    SendMessageResponse,
    _$SendMessageResponse
  ];

  @override
  final String wireName = r'SendMessageResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SendMessageResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.message != null) {
      yield r'message';
      yield serializers.serialize(
        object.message,
        specifiedType: const FullType.nullable(Message),
      );
    }
    if (object.task != null) {
      yield r'task';
      yield serializers.serialize(
        object.task,
        specifiedType: const FullType.nullable(Task),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    SendMessageResponse object, {
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
    required SendMessageResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(Message),
          ) as Message?;
          if (valueDes == null) continue;
          result.message.replace(valueDes);
          break;
        case r'task':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(Task),
          ) as Task?;
          if (valueDes == null) continue;
          result.task.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SendMessageResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SendMessageResponseBuilder();
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
