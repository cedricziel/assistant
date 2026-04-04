//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_conversation_request.g.dart';

/// Body for `POST /api/conversations`.
///
/// Properties:
/// * [title] - Optional title for the new conversation.
@BuiltValue()
abstract class CreateConversationRequest implements Built<CreateConversationRequest, CreateConversationRequestBuilder> {
  /// Optional title for the new conversation.
  @BuiltValueField(wireName: r'title')
  String? get title;

  CreateConversationRequest._();

  factory CreateConversationRequest([void updates(CreateConversationRequestBuilder b)]) = _$CreateConversationRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateConversationRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateConversationRequest> get serializer => _$CreateConversationRequestSerializer();
}

class _$CreateConversationRequestSerializer implements PrimitiveSerializer<CreateConversationRequest> {
  @override
  final Iterable<Type> types = const [CreateConversationRequest, _$CreateConversationRequest];

  @override
  final String wireName = r'CreateConversationRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateConversationRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.title != null) {
      yield r'title';
      yield serializers.serialize(
        object.title,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateConversationRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required CreateConversationRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'title':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.title = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CreateConversationRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateConversationRequestBuilder();
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

