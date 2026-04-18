//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/mutual_tls_security_scheme.dart';
import 'package:assistant_api/src/model/api_key_security_scheme.dart';
import 'package:assistant_api/src/model/open_id_connect_security_scheme.dart';
import 'package:assistant_api/src/model/o_auth2_security_scheme.dart';
import 'package:assistant_api/src/model/http_auth_security_scheme.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'security_scheme.g.dart';

/// Defines a security scheme that can be used to secure an agent's endpoints.  This is a discriminated union based on the OpenAPI 3.2 Security Scheme Object.
///
/// Properties:
/// * [apiKeySecurityScheme] - API key-based authentication.
/// * [httpAuthSecurityScheme] - HTTP authentication (Basic, Bearer, etc.).
/// * [mtlsSecurityScheme] - Mutual TLS authentication.
/// * [oauth2SecurityScheme] - OAuth 2.0 authentication.
/// * [openIdConnectSecurityScheme] - OpenID Connect authentication.
@BuiltValue()
abstract class SecurityScheme
    implements Built<SecurityScheme, SecuritySchemeBuilder> {
  /// API key-based authentication.
  @BuiltValueField(wireName: r'apiKeySecurityScheme')
  ApiKeySecurityScheme? get apiKeySecurityScheme;

  /// HTTP authentication (Basic, Bearer, etc.).
  @BuiltValueField(wireName: r'httpAuthSecurityScheme')
  HttpAuthSecurityScheme? get httpAuthSecurityScheme;

  /// Mutual TLS authentication.
  @BuiltValueField(wireName: r'mtlsSecurityScheme')
  MutualTlsSecurityScheme? get mtlsSecurityScheme;

  /// OAuth 2.0 authentication.
  @BuiltValueField(wireName: r'oauth2SecurityScheme')
  OAuth2SecurityScheme? get oauth2SecurityScheme;

  /// OpenID Connect authentication.
  @BuiltValueField(wireName: r'openIdConnectSecurityScheme')
  OpenIdConnectSecurityScheme? get openIdConnectSecurityScheme;

  SecurityScheme._();

  factory SecurityScheme([void updates(SecuritySchemeBuilder b)]) =
      _$SecurityScheme;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SecuritySchemeBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SecurityScheme> get serializer =>
      _$SecuritySchemeSerializer();
}

class _$SecuritySchemeSerializer
    implements PrimitiveSerializer<SecurityScheme> {
  @override
  final Iterable<Type> types = const [SecurityScheme, _$SecurityScheme];

  @override
  final String wireName = r'SecurityScheme';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SecurityScheme object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.apiKeySecurityScheme != null) {
      yield r'apiKeySecurityScheme';
      yield serializers.serialize(
        object.apiKeySecurityScheme,
        specifiedType: const FullType.nullable(ApiKeySecurityScheme),
      );
    }
    if (object.httpAuthSecurityScheme != null) {
      yield r'httpAuthSecurityScheme';
      yield serializers.serialize(
        object.httpAuthSecurityScheme,
        specifiedType: const FullType.nullable(HttpAuthSecurityScheme),
      );
    }
    if (object.mtlsSecurityScheme != null) {
      yield r'mtlsSecurityScheme';
      yield serializers.serialize(
        object.mtlsSecurityScheme,
        specifiedType: const FullType.nullable(MutualTlsSecurityScheme),
      );
    }
    if (object.oauth2SecurityScheme != null) {
      yield r'oauth2SecurityScheme';
      yield serializers.serialize(
        object.oauth2SecurityScheme,
        specifiedType: const FullType.nullable(OAuth2SecurityScheme),
      );
    }
    if (object.openIdConnectSecurityScheme != null) {
      yield r'openIdConnectSecurityScheme';
      yield serializers.serialize(
        object.openIdConnectSecurityScheme,
        specifiedType: const FullType.nullable(OpenIdConnectSecurityScheme),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    SecurityScheme object, {
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
    required SecuritySchemeBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'apiKeySecurityScheme':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(ApiKeySecurityScheme),
          ) as ApiKeySecurityScheme?;
          if (valueDes == null) continue;
          result.apiKeySecurityScheme.replace(valueDes);
          break;
        case r'httpAuthSecurityScheme':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(HttpAuthSecurityScheme),
          ) as HttpAuthSecurityScheme?;
          if (valueDes == null) continue;
          result.httpAuthSecurityScheme.replace(valueDes);
          break;
        case r'mtlsSecurityScheme':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(MutualTlsSecurityScheme),
          ) as MutualTlsSecurityScheme?;
          if (valueDes == null) continue;
          result.mtlsSecurityScheme.replace(valueDes);
          break;
        case r'oauth2SecurityScheme':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(OAuth2SecurityScheme),
          ) as OAuth2SecurityScheme?;
          if (valueDes == null) continue;
          result.oauth2SecurityScheme.replace(valueDes);
          break;
        case r'openIdConnectSecurityScheme':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(OpenIdConnectSecurityScheme),
          ) as OpenIdConnectSecurityScheme?;
          if (valueDes == null) continue;
          result.openIdConnectSecurityScheme.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SecurityScheme deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SecuritySchemeBuilder();
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
