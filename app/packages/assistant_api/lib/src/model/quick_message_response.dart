//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'quick_message_response.g.dart';

/// Response for `POST /api/quick-message`.
///
/// Properties:
/// * [answer] - The full assistant response text.
/// * [conversationId] - The ID of the newly created conversation.
/// * [messageId] - The ID of the persisted assistant message, if available.
@BuiltValue()
abstract class QuickMessageResponse implements Built<QuickMessageResponse, QuickMessageResponseBuilder> {
  /// The full assistant response text.
  @BuiltValueField(wireName: r'answer')
  String get answer;

  /// The ID of the newly created conversation.
  @BuiltValueField(wireName: r'conversation_id')
  String get conversationId;

  /// The ID of the persisted assistant message, if available.
  @BuiltValueField(wireName: r'message_id')
  String? get messageId;

  QuickMessageResponse._();

  factory QuickMessageResponse([void updates(QuickMessageResponseBuilder b)]) = _$QuickMessageResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(QuickMessageResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<QuickMessageResponse> get serializer => _$QuickMessageResponseSerializer();
}

class _$QuickMessageResponseSerializer implements PrimitiveSerializer<QuickMessageResponse> {
  @override
  final Iterable<Type> types = const [QuickMessageResponse, _$QuickMessageResponse];

  @override
  final String wireName = r'QuickMessageResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    QuickMessageResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'answer';
    yield serializers.serialize(
      object.answer,
      specifiedType: const FullType(String),
    );
    yield r'conversation_id';
    yield serializers.serialize(
      object.conversationId,
      specifiedType: const FullType(String),
    );
    if (object.messageId != null) {
      yield r'message_id';
      yield serializers.serialize(
        object.messageId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    QuickMessageResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required QuickMessageResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'answer':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.answer = valueDes;
          break;
        case r'conversation_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.conversationId = valueDes;
          break;
        case r'message_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.messageId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  QuickMessageResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = QuickMessageResponseBuilder();
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

