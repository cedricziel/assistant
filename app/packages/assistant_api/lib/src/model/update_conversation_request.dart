//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_conversation_request.g.dart';

/// Body for `PATCH /api/conversations/{id}`.
///
/// Properties:
/// * [title] - New title for the conversation.
@BuiltValue()
abstract class UpdateConversationRequest
    implements
        Built<UpdateConversationRequest, UpdateConversationRequestBuilder> {
  /// New title for the conversation.
  @BuiltValueField(wireName: r'title')
  String get title;

  UpdateConversationRequest._();

  factory UpdateConversationRequest(
          [void updates(UpdateConversationRequestBuilder b)]) =
      _$UpdateConversationRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateConversationRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateConversationRequest> get serializer =>
      _$UpdateConversationRequestSerializer();
}

class _$UpdateConversationRequestSerializer
    implements PrimitiveSerializer<UpdateConversationRequest> {
  @override
  final Iterable<Type> types = const [
    UpdateConversationRequest,
    _$UpdateConversationRequest
  ];

  @override
  final String wireName = r'UpdateConversationRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateConversationRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'title';
    yield serializers.serialize(
      object.title,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdateConversationRequest object, {
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
    required UpdateConversationRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'title':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
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
  UpdateConversationRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateConversationRequestBuilder();
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
