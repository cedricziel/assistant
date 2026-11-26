//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/feedback_rating.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'set_message_feedback_request.g.dart';

/// Body for `PUT /api/conversations/{id}/messages/{message_id}/feedback`.
///
/// Properties:
/// * [comment] - Optional free-text reason. Trimmed; an empty result means no comment. At most 2000 Unicode scalar values. Omitting it clears a stored comment.
/// * [rating] - Thumbs up or down.
@BuiltValue()
abstract class SetMessageFeedbackRequest
    implements
        Built<SetMessageFeedbackRequest, SetMessageFeedbackRequestBuilder> {
  /// Optional free-text reason. Trimmed; an empty result means no comment. At most 2000 Unicode scalar values. Omitting it clears a stored comment.
  @BuiltValueField(wireName: r'comment')
  String? get comment;

  /// Thumbs up or down.
  @BuiltValueField(wireName: r'rating')
  FeedbackRating get rating;
  // enum ratingEnum {  positive,  negative,  };

  SetMessageFeedbackRequest._();

  factory SetMessageFeedbackRequest(
          [void updates(SetMessageFeedbackRequestBuilder b)]) =
      _$SetMessageFeedbackRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SetMessageFeedbackRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SetMessageFeedbackRequest> get serializer =>
      _$SetMessageFeedbackRequestSerializer();
}

class _$SetMessageFeedbackRequestSerializer
    implements PrimitiveSerializer<SetMessageFeedbackRequest> {
  @override
  final Iterable<Type> types = const [
    SetMessageFeedbackRequest,
    _$SetMessageFeedbackRequest
  ];

  @override
  final String wireName = r'SetMessageFeedbackRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SetMessageFeedbackRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.comment != null) {
      yield r'comment';
      yield serializers.serialize(
        object.comment,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'rating';
    yield serializers.serialize(
      object.rating,
      specifiedType: const FullType(FeedbackRating),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SetMessageFeedbackRequest object, {
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
    required SetMessageFeedbackRequestBuilder result,
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
        case r'rating':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(FeedbackRating),
          ) as FeedbackRating;
          result.rating = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SetMessageFeedbackRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SetMessageFeedbackRequestBuilder();
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
