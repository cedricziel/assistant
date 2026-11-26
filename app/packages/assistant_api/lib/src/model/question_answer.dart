//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'question_answer.g.dart';

/// The owner's answer to one question of a `question` interaction.
///
/// Properties:
/// * [questionId] - Id of the question being answered.
/// * [selectedOptionIds] - Ids of the chosen options. Empty when the answer is free text only.
/// * [text] - Free-text answer, at most 4000 characters, where the question allows it.
@BuiltValue()
abstract class QuestionAnswer
    implements Built<QuestionAnswer, QuestionAnswerBuilder> {
  /// Id of the question being answered.
  @BuiltValueField(wireName: r'question_id')
  String get questionId;

  /// Ids of the chosen options. Empty when the answer is free text only.
  @BuiltValueField(wireName: r'selected_option_ids')
  BuiltList<String>? get selectedOptionIds;

  /// Free-text answer, at most 4000 characters, where the question allows it.
  @BuiltValueField(wireName: r'text')
  String? get text;

  QuestionAnswer._();

  factory QuestionAnswer([void updates(QuestionAnswerBuilder b)]) =
      _$QuestionAnswer;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(QuestionAnswerBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<QuestionAnswer> get serializer =>
      _$QuestionAnswerSerializer();
}

class _$QuestionAnswerSerializer
    implements PrimitiveSerializer<QuestionAnswer> {
  @override
  final Iterable<Type> types = const [QuestionAnswer, _$QuestionAnswer];

  @override
  final String wireName = r'QuestionAnswer';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    QuestionAnswer object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'question_id';
    yield serializers.serialize(
      object.questionId,
      specifiedType: const FullType(String),
    );
    if (object.selectedOptionIds != null) {
      yield r'selected_option_ids';
      yield serializers.serialize(
        object.selectedOptionIds,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    if (object.text != null) {
      yield r'text';
      yield serializers.serialize(
        object.text,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    QuestionAnswer object, {
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
    required QuestionAnswerBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'question_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.questionId = valueDes;
          break;
        case r'selected_option_ids':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.selectedOptionIds.replace(valueDes);
          break;
        case r'text':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.text = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  QuestionAnswer deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = QuestionAnswerBuilder();
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
