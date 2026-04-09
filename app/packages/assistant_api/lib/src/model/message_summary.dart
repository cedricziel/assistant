//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'message_summary.g.dart';

/// A single message in a conversation.
///
/// Properties:
/// * [content] 
/// * [createdAt] 
/// * [id] 
/// * [role] 
/// * [skillName] - Name of the tool or skill that produced this result (present when `role == \"tool\"`).
/// * [toolCalls] - Tool names called in this message (present when `role == \"assistant\"` and the message contains tool invocations).
/// * [turn] 
@BuiltValue()
abstract class MessageSummary implements Built<MessageSummary, MessageSummaryBuilder> {
  @BuiltValueField(wireName: r'content')
  String get content;

  @BuiltValueField(wireName: r'created_at')
  DateTime get createdAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'role')
  String get role;

  /// Name of the tool or skill that produced this result (present when `role == \"tool\"`).
  @BuiltValueField(wireName: r'skill_name')
  String? get skillName;

  /// Tool names called in this message (present when `role == \"assistant\"` and the message contains tool invocations).
  @BuiltValueField(wireName: r'tool_calls')
  BuiltList<String>? get toolCalls;

  @BuiltValueField(wireName: r'turn')
  int get turn;

  MessageSummary._();

  factory MessageSummary([void updates(MessageSummaryBuilder b)]) = _$MessageSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(MessageSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<MessageSummary> get serializer => _$MessageSummarySerializer();
}

class _$MessageSummarySerializer implements PrimitiveSerializer<MessageSummary> {
  @override
  final Iterable<Type> types = const [MessageSummary, _$MessageSummary];

  @override
  final String wireName = r'MessageSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    MessageSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'content';
    yield serializers.serialize(
      object.content,
      specifiedType: const FullType(String),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'role';
    yield serializers.serialize(
      object.role,
      specifiedType: const FullType(String),
    );
    if (object.skillName != null) {
      yield r'skill_name';
      yield serializers.serialize(
        object.skillName,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.toolCalls != null) {
      yield r'tool_calls';
      yield serializers.serialize(
        object.toolCalls,
        specifiedType: const FullType.nullable(BuiltList, [FullType(String)]),
      );
    }
    yield r'turn';
    yield serializers.serialize(
      object.turn,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    MessageSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required MessageSummaryBuilder result,
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
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.createdAt = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'role':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.role = valueDes;
          break;
        case r'skill_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.skillName = valueDes;
          break;
        case r'tool_calls':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(BuiltList, [FullType(String)]),
          ) as BuiltList<String>?;
          if (valueDes == null) continue;
          result.toolCalls.replace(valueDes);
          break;
        case r'turn':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.turn = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  MessageSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = MessageSummaryBuilder();
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

