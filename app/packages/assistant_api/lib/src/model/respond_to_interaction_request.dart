//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/tool_decision.dart';
import 'package:assistant_api/src/model/question_answer.dart';
import 'package:assistant_api/src/model/respond_to_interaction_request_one_of2.dart';
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/respond_to_interaction_request_one_of1.dart';
import 'package:assistant_api/src/model/respond_to_interaction_request_one_of.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';
import 'package:one_of/one_of.dart';

part 'respond_to_interaction_request.g.dart';

/// Body of the respond route, tagged by `kind`. Unknown fields are rejected, so a client cannot smuggle in a field such as a permission rule: the saved rule comes from the stored interaction, never from the body.
///
/// Properties:
/// * [decision]
/// * [kind]
/// * [answers]
@BuiltValue()
abstract class RespondToInteractionRequest
    implements
        Built<RespondToInteractionRequest, RespondToInteractionRequestBuilder> {
  /// One Of [RespondToInteractionRequestOneOf], [RespondToInteractionRequestOneOf1], [RespondToInteractionRequestOneOf2]
  OneOf get oneOf;

  RespondToInteractionRequest._();

  factory RespondToInteractionRequest(
          [void updates(RespondToInteractionRequestBuilder b)]) =
      _$RespondToInteractionRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RespondToInteractionRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RespondToInteractionRequest> get serializer =>
      _$RespondToInteractionRequestSerializer();
}

class _$RespondToInteractionRequestSerializer
    implements PrimitiveSerializer<RespondToInteractionRequest> {
  @override
  final Iterable<Type> types = const [
    RespondToInteractionRequest,
    _$RespondToInteractionRequest
  ];

  @override
  final String wireName = r'RespondToInteractionRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RespondToInteractionRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {}

  @override
  Object serialize(
    Serializers serializers,
    RespondToInteractionRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final oneOf = object.oneOf;
    return serializers.serialize(oneOf.value,
        specifiedType: FullType(oneOf.valueType))!;
  }

  @override
  RespondToInteractionRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RespondToInteractionRequestBuilder();
    Object? oneOfDataSrc;
    final targetType = const FullType(OneOf, [
      FullType(RespondToInteractionRequestOneOf),
      FullType(RespondToInteractionRequestOneOf1),
      FullType(RespondToInteractionRequestOneOf2),
    ]);
    oneOfDataSrc = serialized;
    result.oneOf = serializers.deserialize(oneOfDataSrc,
        specifiedType: targetType) as OneOf;
    return result.build();
  }
}

class RespondToInteractionRequestKindEnum extends EnumClass {
  @BuiltValueEnumConst(wireName: r'decline')
  static const RespondToInteractionRequestKindEnum decline =
      _$respondToInteractionRequestKindEnum_decline;
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const RespondToInteractionRequestKindEnum unknownDefaultOpenApi =
      _$respondToInteractionRequestKindEnum_unknownDefaultOpenApi;

  static Serializer<RespondToInteractionRequestKindEnum> get serializer =>
      _$respondToInteractionRequestKindEnumSerializer;

  const RespondToInteractionRequestKindEnum._(String name) : super(name);

  static BuiltSet<RespondToInteractionRequestKindEnum> get values =>
      _$respondToInteractionRequestKindEnumValues;
  static RespondToInteractionRequestKindEnum valueOf(String name) =>
      _$respondToInteractionRequestKindEnumValueOf(name);
}
