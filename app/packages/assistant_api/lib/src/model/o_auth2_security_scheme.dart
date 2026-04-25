//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/o_auth_flows.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'o_auth2_security_scheme.g.dart';

/// Defines a security scheme using OAuth 2.0.
///
/// Properties:
/// * [description] - An optional description for the security scheme.
/// * [flows] - Configuration for the supported OAuth 2.0 flows.
/// * [oauth2MetadataUrl] - URL to the OAuth2 authorization server metadata (RFC 8414).
@BuiltValue()
abstract class OAuth2SecurityScheme
    implements Built<OAuth2SecurityScheme, OAuth2SecuritySchemeBuilder> {
  /// An optional description for the security scheme.
  @BuiltValueField(wireName: r'description')
  String? get description;

  /// Configuration for the supported OAuth 2.0 flows.
  @BuiltValueField(wireName: r'flows')
  OAuthFlows get flows;

  /// URL to the OAuth2 authorization server metadata (RFC 8414).
  @BuiltValueField(wireName: r'oauth2MetadataUrl')
  String? get oauth2MetadataUrl;

  OAuth2SecurityScheme._();

  factory OAuth2SecurityScheme([void updates(OAuth2SecuritySchemeBuilder b)]) =
      _$OAuth2SecurityScheme;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(OAuth2SecuritySchemeBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<OAuth2SecurityScheme> get serializer =>
      _$OAuth2SecuritySchemeSerializer();
}

class _$OAuth2SecuritySchemeSerializer
    implements PrimitiveSerializer<OAuth2SecurityScheme> {
  @override
  final Iterable<Type> types = const [
    OAuth2SecurityScheme,
    _$OAuth2SecurityScheme
  ];

  @override
  final String wireName = r'OAuth2SecurityScheme';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    OAuth2SecurityScheme object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.description != null) {
      yield r'description';
      yield serializers.serialize(
        object.description,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'flows';
    yield serializers.serialize(
      object.flows,
      specifiedType: const FullType(OAuthFlows),
    );
    if (object.oauth2MetadataUrl != null) {
      yield r'oauth2MetadataUrl';
      yield serializers.serialize(
        object.oauth2MetadataUrl,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    OAuth2SecurityScheme object, {
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
    required OAuth2SecuritySchemeBuilder result,
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
        case r'flows':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(OAuthFlows),
          ) as OAuthFlows;
          result.flows.replace(valueDes);
          break;
        case r'oauth2MetadataUrl':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.oauth2MetadataUrl = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  OAuth2SecurityScheme deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = OAuth2SecuritySchemeBuilder();
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
