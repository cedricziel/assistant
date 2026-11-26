//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/feedback_rating.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'message_feedback_response.g.dart';

/// The caller's stored feedback on a message.
///
/// Properties:
/// * [comment]
/// * [createdAt]
/// * [messageId]
/// * [rating]
/// * [updatedAt]
@BuiltValue()
abstract class MessageFeedbackResponse
    implements Built<MessageFeedbackResponse, MessageFeedbackResponseBuilder> {
  @BuiltValueField(wireName: r'comment')
  String? get comment;

  @BuiltValueField(wireName: r'created_at')
  DateTime get createdAt;

  @BuiltValueField(wireName: r'message_id')
  String get messageId;

  @BuiltValueField(wireName: r'rating')
  FeedbackRating get rating;
  // enum ratingEnum {  positive,  negative,  };

  @BuiltValueField(wireName: r'updated_at')
  DateTime get updatedAt;

  MessageFeedbackResponse._();

  factory MessageFeedbackResponse(
          [void updates(MessageFeedbackResponseBuilder b)]) =
      _$MessageFeedbackResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(MessageFeedbackResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<MessageFeedbackResponse> get serializer =>
      _$MessageFeedbackResponseSerializer();
}

class _$MessageFeedbackResponseSerializer
    implements PrimitiveSerializer<MessageFeedbackResponse> {
  @override
  final Iterable<Type> types = const [
    MessageFeedbackResponse,
    _$MessageFeedbackResponse
  ];

  @override
  final String wireName = r'MessageFeedbackResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    MessageFeedbackResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.comment != null) {
      yield r'comment';
      yield serializers.serialize(
        object.comment,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'message_id';
    yield serializers.serialize(
      object.messageId,
      specifiedType: const FullType(String),
    );
    yield r'rating';
    yield serializers.serialize(
      object.rating,
      specifiedType: const FullType(FeedbackRating),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(DateTime),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    MessageFeedbackResponse object, {
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
    required MessageFeedbackResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'comment':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.comment = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.createdAt = valueDes;
          break;
        case r'message_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.messageId = valueDes;
          break;
        case r'rating':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(FeedbackRating),
          ) as FeedbackRating;
          result.rating = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.updatedAt = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  MessageFeedbackResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = MessageFeedbackResponseBuilder();
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
