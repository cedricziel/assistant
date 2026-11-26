//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/question_answer.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'respond_to_interaction_request_one_of1.g.dart';

/// Answers a `question` interaction.
///
/// Properties:
/// * [answers]
/// * [kind]
@BuiltValue()
abstract class RespondToInteractionRequestOneOf1
    implements
        Built<RespondToInteractionRequestOneOf1,
            RespondToInteractionRequestOneOf1Builder> {
  @BuiltValueField(wireName: r'answers')
  BuiltList<QuestionAnswer> get answers;

  @BuiltValueField(wireName: r'kind')
  RespondToInteractionRequestOneOf1KindEnum get kind;
  // enum kindEnum {  question,  };

  RespondToInteractionRequestOneOf1._();

  factory RespondToInteractionRequestOneOf1(
          [void updates(RespondToInteractionRequestOneOf1Builder b)]) =
      _$RespondToInteractionRequestOneOf1;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RespondToInteractionRequestOneOf1Builder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RespondToInteractionRequestOneOf1> get serializer =>
      _$RespondToInteractionRequestOneOf1Serializer();
}

class _$RespondToInteractionRequestOneOf1Serializer
    implements PrimitiveSerializer<RespondToInteractionRequestOneOf1> {
  @override
  final Iterable<Type> types = const [
    RespondToInteractionRequestOneOf1,
    _$RespondToInteractionRequestOneOf1
  ];

  @override
  final String wireName = r'RespondToInteractionRequestOneOf1';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RespondToInteractionRequestOneOf1 object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'answers';
    yield serializers.serialize(
      object.answers,
      specifiedType: const FullType(BuiltList, [FullType(QuestionAnswer)]),
    );
    yield r'kind';
    yield serializers.serialize(
      object.kind,
      specifiedType: const FullType(RespondToInteractionRequestOneOf1KindEnum),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RespondToInteractionRequestOneOf1 object, {
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
    required RespondToInteractionRequestOneOf1Builder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'answers':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(BuiltList, [FullType(QuestionAnswer)]),
          ) as BuiltList<QuestionAnswer>;
          result.answers.replace(valueDes);
          break;
        case r'kind':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(RespondToInteractionRequestOneOf1KindEnum),
          ) as RespondToInteractionRequestOneOf1KindEnum;
          result.kind = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RespondToInteractionRequestOneOf1 deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RespondToInteractionRequestOneOf1Builder();
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

class RespondToInteractionRequestOneOf1KindEnum extends EnumClass {
  @BuiltValueEnumConst(wireName: r'question')
  static const RespondToInteractionRequestOneOf1KindEnum question =
      _$respondToInteractionRequestOneOf1KindEnum_question;
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const RespondToInteractionRequestOneOf1KindEnum unknownDefaultOpenApi =
      _$respondToInteractionRequestOneOf1KindEnum_unknownDefaultOpenApi;

  static Serializer<RespondToInteractionRequestOneOf1KindEnum> get serializer =>
      _$respondToInteractionRequestOneOf1KindEnumSerializer;

  const RespondToInteractionRequestOneOf1KindEnum._(String name) : super(name);

  static BuiltSet<RespondToInteractionRequestOneOf1KindEnum> get values =>
      _$respondToInteractionRequestOneOf1KindEnumValues;
  static RespondToInteractionRequestOneOf1KindEnum valueOf(String name) =>
      _$respondToInteractionRequestOneOf1KindEnumValueOf(name);
}
