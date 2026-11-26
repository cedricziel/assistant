//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'message_feedback_summary.g.dart';

/// The calling user's feedback on a message, as embedded in history.
///
/// Properties:
/// * [comment] - Optional free-text comment. Omitted when none was given.
/// * [rating] - `\"positive\"` or `\"negative\"`.
/// * [updatedAt]
@BuiltValue()
abstract class MessageFeedbackSummary
    implements Built<MessageFeedbackSummary, MessageFeedbackSummaryBuilder> {
  /// Optional free-text comment. Omitted when none was given.
  @BuiltValueField(wireName: r'comment')
  String? get comment;

  /// `\"positive\"` or `\"negative\"`.
  @BuiltValueField(wireName: r'rating')
  String get rating;

  @BuiltValueField(wireName: r'updated_at')
  DateTime get updatedAt;

  MessageFeedbackSummary._();

  factory MessageFeedbackSummary(
          [void updates(MessageFeedbackSummaryBuilder b)]) =
      _$MessageFeedbackSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(MessageFeedbackSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<MessageFeedbackSummary> get serializer =>
      _$MessageFeedbackSummarySerializer();
}

class _$MessageFeedbackSummarySerializer
    implements PrimitiveSerializer<MessageFeedbackSummary> {
  @override
  final Iterable<Type> types = const [
    MessageFeedbackSummary,
    _$MessageFeedbackSummary
  ];

  @override
  final String wireName = r'MessageFeedbackSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    MessageFeedbackSummary object, {
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
      specifiedType: const FullType(String),
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
    MessageFeedbackSummary object, {
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
    required MessageFeedbackSummaryBuilder result,
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
            specifiedType: const FullType(String),
          ) as String;
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
  MessageFeedbackSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = MessageFeedbackSummaryBuilder();
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
