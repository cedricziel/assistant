//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/tool_decision.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'respond_to_interaction_request_one_of.g.dart';

/// Answers a `tool_approval` interaction with one of its offered options.
///
/// Properties:
/// * [decision]
/// * [kind]
@BuiltValue()
abstract class RespondToInteractionRequestOneOf
    implements
        Built<RespondToInteractionRequestOneOf,
            RespondToInteractionRequestOneOfBuilder> {
  @BuiltValueField(wireName: r'decision')
  ToolDecision get decision;
  // enum decisionEnum {  allow_once,  allow_always,  reject_once,  reject_always,  };

  @BuiltValueField(wireName: r'kind')
  RespondToInteractionRequestOneOfKindEnum get kind;
  // enum kindEnum {  tool_approval,  };

  RespondToInteractionRequestOneOf._();

  factory RespondToInteractionRequestOneOf(
          [void updates(RespondToInteractionRequestOneOfBuilder b)]) =
      _$RespondToInteractionRequestOneOf;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RespondToInteractionRequestOneOfBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RespondToInteractionRequestOneOf> get serializer =>
      _$RespondToInteractionRequestOneOfSerializer();
}

class _$RespondToInteractionRequestOneOfSerializer
    implements PrimitiveSerializer<RespondToInteractionRequestOneOf> {
  @override
  final Iterable<Type> types = const [
    RespondToInteractionRequestOneOf,
    _$RespondToInteractionRequestOneOf
  ];

  @override
  final String wireName = r'RespondToInteractionRequestOneOf';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RespondToInteractionRequestOneOf object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'decision';
    yield serializers.serialize(
      object.decision,
      specifiedType: const FullType(ToolDecision),
    );
    yield r'kind';
    yield serializers.serialize(
      object.kind,
      specifiedType: const FullType(RespondToInteractionRequestOneOfKindEnum),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RespondToInteractionRequestOneOf object, {
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
    required RespondToInteractionRequestOneOfBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'decision':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(ToolDecision),
          ) as ToolDecision;
          result.decision = valueDes;
          break;
        case r'kind':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(RespondToInteractionRequestOneOfKindEnum),
          ) as RespondToInteractionRequestOneOfKindEnum;
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
  RespondToInteractionRequestOneOf deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RespondToInteractionRequestOneOfBuilder();
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

class RespondToInteractionRequestOneOfKindEnum extends EnumClass {
  @BuiltValueEnumConst(wireName: r'tool_approval')
  static const RespondToInteractionRequestOneOfKindEnum toolApproval =
      _$respondToInteractionRequestOneOfKindEnum_toolApproval;
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const RespondToInteractionRequestOneOfKindEnum unknownDefaultOpenApi =
      _$respondToInteractionRequestOneOfKindEnum_unknownDefaultOpenApi;

  static Serializer<RespondToInteractionRequestOneOfKindEnum> get serializer =>
      _$respondToInteractionRequestOneOfKindEnumSerializer;

  const RespondToInteractionRequestOneOfKindEnum._(String name) : super(name);

  static BuiltSet<RespondToInteractionRequestOneOfKindEnum> get values =>
      _$respondToInteractionRequestOneOfKindEnumValues;
  static RespondToInteractionRequestOneOfKindEnum valueOf(String name) =>
      _$respondToInteractionRequestOneOfKindEnumValueOf(name);
}
