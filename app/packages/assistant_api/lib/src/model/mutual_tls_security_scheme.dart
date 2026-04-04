//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'mutual_tls_security_scheme.g.dart';

/// Defines a security scheme using mTLS authentication.
///
/// Properties:
/// * [description] - An optional description for the security scheme.
@BuiltValue()
abstract class MutualTlsSecurityScheme implements Built<MutualTlsSecurityScheme, MutualTlsSecuritySchemeBuilder> {
  /// An optional description for the security scheme.
  @BuiltValueField(wireName: r'description')
  String? get description;

  MutualTlsSecurityScheme._();

  factory MutualTlsSecurityScheme([void updates(MutualTlsSecuritySchemeBuilder b)]) = _$MutualTlsSecurityScheme;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(MutualTlsSecuritySchemeBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<MutualTlsSecurityScheme> get serializer => _$MutualTlsSecuritySchemeSerializer();
}

class _$MutualTlsSecuritySchemeSerializer implements PrimitiveSerializer<MutualTlsSecurityScheme> {
  @override
  final Iterable<Type> types = const [MutualTlsSecurityScheme, _$MutualTlsSecurityScheme];

  @override
  final String wireName = r'MutualTlsSecurityScheme';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    MutualTlsSecurityScheme object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.description != null) {
      yield r'description';
      yield serializers.serialize(
        object.description,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    MutualTlsSecurityScheme object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required MutualTlsSecuritySchemeBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.description = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  MutualTlsSecurityScheme deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = MutualTlsSecuritySchemeBuilder();
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

