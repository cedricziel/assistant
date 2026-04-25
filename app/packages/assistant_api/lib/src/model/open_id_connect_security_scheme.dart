//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'open_id_connect_security_scheme.g.dart';

/// Defines a security scheme using OpenID Connect.
///
/// Properties:
/// * [description] - An optional description for the security scheme.
/// * [openIdConnectUrl] - The OpenID Connect Discovery URL.
@BuiltValue()
abstract class OpenIdConnectSecurityScheme
    implements
        Built<OpenIdConnectSecurityScheme, OpenIdConnectSecuritySchemeBuilder> {
  /// An optional description for the security scheme.
  @BuiltValueField(wireName: r'description')
  String? get description;

  /// The OpenID Connect Discovery URL.
  @BuiltValueField(wireName: r'openIdConnectUrl')
  String get openIdConnectUrl;

  OpenIdConnectSecurityScheme._();

  factory OpenIdConnectSecurityScheme(
          [void updates(OpenIdConnectSecuritySchemeBuilder b)]) =
      _$OpenIdConnectSecurityScheme;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(OpenIdConnectSecuritySchemeBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<OpenIdConnectSecurityScheme> get serializer =>
      _$OpenIdConnectSecuritySchemeSerializer();
}

class _$OpenIdConnectSecuritySchemeSerializer
    implements PrimitiveSerializer<OpenIdConnectSecurityScheme> {
  @override
  final Iterable<Type> types = const [
    OpenIdConnectSecurityScheme,
    _$OpenIdConnectSecurityScheme
  ];

  @override
  final String wireName = r'OpenIdConnectSecurityScheme';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    OpenIdConnectSecurityScheme object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.description != null) {
      yield r'description';
      yield serializers.serialize(
        object.description,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'openIdConnectUrl';
    yield serializers.serialize(
      object.openIdConnectUrl,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    OpenIdConnectSecurityScheme object, {
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
    required OpenIdConnectSecuritySchemeBuilder result,
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
        case r'openIdConnectUrl':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.openIdConnectUrl = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  OpenIdConnectSecurityScheme deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = OpenIdConnectSecuritySchemeBuilder();
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
