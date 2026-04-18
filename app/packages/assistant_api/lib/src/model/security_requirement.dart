//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/string_list.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'security_requirement.g.dart';

/// Defines the security requirements for an agent.
///
/// Properties:
/// * [schemes] - A map of security schemes to the required scopes.
@BuiltValue()
abstract class SecurityRequirement
    implements Built<SecurityRequirement, SecurityRequirementBuilder> {
  /// A map of security schemes to the required scopes.
  @BuiltValueField(wireName: r'schemes')
  BuiltMap<String, StringList>? get schemes;

  SecurityRequirement._();

  factory SecurityRequirement([void updates(SecurityRequirementBuilder b)]) =
      _$SecurityRequirement;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SecurityRequirementBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SecurityRequirement> get serializer =>
      _$SecurityRequirementSerializer();
}

class _$SecurityRequirementSerializer
    implements PrimitiveSerializer<SecurityRequirement> {
  @override
  final Iterable<Type> types = const [
    SecurityRequirement,
    _$SecurityRequirement
  ];

  @override
  final String wireName = r'SecurityRequirement';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SecurityRequirement object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.schemes != null) {
      yield r'schemes';
      yield serializers.serialize(
        object.schemes,
        specifiedType:
            const FullType(BuiltMap, [FullType(String), FullType(StringList)]),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    SecurityRequirement object, {
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
    required SecurityRequirementBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'schemes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(
                BuiltMap, [FullType(String), FullType(StringList)]),
          ) as BuiltMap<String, StringList>;
          result.schemes.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SecurityRequirement deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SecurityRequirementBuilder();
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
