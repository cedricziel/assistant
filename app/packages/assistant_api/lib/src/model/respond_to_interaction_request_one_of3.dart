//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'respond_to_interaction_request_one_of3.g.dart';

/// \"I won't answer\", for any kind. For an approval this means reject once.
///
/// Properties:
/// * [kind]
@BuiltValue()
abstract class RespondToInteractionRequestOneOf3
    implements
        Built<RespondToInteractionRequestOneOf3,
            RespondToInteractionRequestOneOf3Builder> {
  @BuiltValueField(wireName: r'kind')
  RespondToInteractionRequestOneOf3KindEnum get kind;
  // enum kindEnum {  decline,  };

  RespondToInteractionRequestOneOf3._();

  factory RespondToInteractionRequestOneOf3(
          [void updates(RespondToInteractionRequestOneOf3Builder b)]) =
      _$RespondToInteractionRequestOneOf3;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RespondToInteractionRequestOneOf3Builder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RespondToInteractionRequestOneOf3> get serializer =>
      _$RespondToInteractionRequestOneOf3Serializer();
}

class _$RespondToInteractionRequestOneOf3Serializer
    implements PrimitiveSerializer<RespondToInteractionRequestOneOf3> {
  @override
  final Iterable<Type> types = const [
    RespondToInteractionRequestOneOf3,
    _$RespondToInteractionRequestOneOf3
  ];

  @override
  final String wireName = r'RespondToInteractionRequestOneOf3';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RespondToInteractionRequestOneOf3 object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'kind';
    yield serializers.serialize(
      object.kind,
      specifiedType: const FullType(RespondToInteractionRequestOneOf3KindEnum),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RespondToInteractionRequestOneOf3 object, {
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
    required RespondToInteractionRequestOneOf3Builder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'kind':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(RespondToInteractionRequestOneOf3KindEnum),
          ) as RespondToInteractionRequestOneOf3KindEnum;
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
  RespondToInteractionRequestOneOf3 deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RespondToInteractionRequestOneOf3Builder();
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

class RespondToInteractionRequestOneOf3KindEnum extends EnumClass {
  @BuiltValueEnumConst(wireName: r'decline')
  static const RespondToInteractionRequestOneOf3KindEnum decline =
      _$respondToInteractionRequestOneOf3KindEnum_decline;
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const RespondToInteractionRequestOneOf3KindEnum unknownDefaultOpenApi =
      _$respondToInteractionRequestOneOf3KindEnum_unknownDefaultOpenApi;

  static Serializer<RespondToInteractionRequestOneOf3KindEnum> get serializer =>
      _$respondToInteractionRequestOneOf3KindEnumSerializer;

  const RespondToInteractionRequestOneOf3KindEnum._(String name) : super(name);

  static BuiltSet<RespondToInteractionRequestOneOf3KindEnum> get values =>
      _$respondToInteractionRequestOneOf3KindEnumValues;
  static RespondToInteractionRequestOneOf3KindEnum valueOf(String name) =>
      _$respondToInteractionRequestOneOf3KindEnumValueOf(name);
}
