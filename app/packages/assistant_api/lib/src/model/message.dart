//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/model_part.dart';
import 'package:assistant_api/src/model/role.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'message.g.dart';

/// `Message` is one unit of communication between client and server.
///
/// Properties:
/// * [contextId] - The context id of the message.
/// * [extensions] - The URIs of extensions present or contributing to this message.
/// * [messageId] - Unique identifier (UUID) of the message, created by the message creator.
/// * [metadata] - Any metadata to provide along with the message.
/// * [parts] - Content parts of the message.
/// * [referenceTaskIds] - A list of task IDs that this message references for additional context.
/// * [role] - Identifies the sender of the message.
/// * [taskId] - The task id of the message.
@BuiltValue()
abstract class Message implements Built<Message, MessageBuilder> {
  /// The context id of the message.
  @BuiltValueField(wireName: r'contextId')
  String? get contextId;

  /// The URIs of extensions present or contributing to this message.
  @BuiltValueField(wireName: r'extensions')
  BuiltList<String>? get extensions;

  /// Unique identifier (UUID) of the message, created by the message creator.
  @BuiltValueField(wireName: r'messageId')
  String get messageId;

  /// Any metadata to provide along with the message.
  @BuiltValueField(wireName: r'metadata')
  BuiltMap<String, JsonObject?>? get metadata;

  /// Content parts of the message.
  @BuiltValueField(wireName: r'parts')
  BuiltList<ModelPart> get parts;

  /// A list of task IDs that this message references for additional context.
  @BuiltValueField(wireName: r'referenceTaskIds')
  BuiltList<String>? get referenceTaskIds;

  /// Identifies the sender of the message.
  @BuiltValueField(wireName: r'role')
  Role get role;
  // enum roleEnum {  ROLE_UNSPECIFIED,  ROLE_USER,  ROLE_AGENT,  };

  /// The task id of the message.
  @BuiltValueField(wireName: r'taskId')
  String? get taskId;

  Message._();

  factory Message([void updates(MessageBuilder b)]) = _$Message;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(MessageBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<Message> get serializer => _$MessageSerializer();
}

class _$MessageSerializer implements PrimitiveSerializer<Message> {
  @override
  final Iterable<Type> types = const [Message, _$Message];

  @override
  final String wireName = r'Message';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    Message object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.contextId != null) {
      yield r'contextId';
      yield serializers.serialize(
        object.contextId,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.extensions != null) {
      yield r'extensions';
      yield serializers.serialize(
        object.extensions,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    yield r'messageId';
    yield serializers.serialize(
      object.messageId,
      specifiedType: const FullType(String),
    );
    if (object.metadata != null) {
      yield r'metadata';
      yield serializers.serialize(
        object.metadata,
        specifiedType: const FullType.nullable(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
      );
    }
    yield r'parts';
    yield serializers.serialize(
      object.parts,
      specifiedType: const FullType(BuiltList, [FullType(ModelPart)]),
    );
    if (object.referenceTaskIds != null) {
      yield r'referenceTaskIds';
      yield serializers.serialize(
        object.referenceTaskIds,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    yield r'role';
    yield serializers.serialize(
      object.role,
      specifiedType: const FullType(Role),
    );
    if (object.taskId != null) {
      yield r'taskId';
      yield serializers.serialize(
        object.taskId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    Message object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required MessageBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'contextId':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.contextId = valueDes;
          break;
        case r'extensions':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.extensions.replace(valueDes);
          break;
        case r'messageId':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.messageId = valueDes;
          break;
        case r'metadata':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
          ) as BuiltMap<String, JsonObject?>?;
          if (valueDes == null) continue;
          result.metadata.replace(valueDes);
          break;
        case r'parts':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ModelPart)]),
          ) as BuiltList<ModelPart>;
          result.parts.replace(valueDes);
          break;
        case r'referenceTaskIds':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.referenceTaskIds.replace(valueDes);
          break;
        case r'role':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(Role),
          ) as Role;
          result.role = valueDes;
          break;
        case r'taskId':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.taskId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  Message deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = MessageBuilder();
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

