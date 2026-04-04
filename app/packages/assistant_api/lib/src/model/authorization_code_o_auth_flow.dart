//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'authorization_code_o_auth_flow.g.dart';

/// OAuth 2.0 Authorization Code flow configuration.
///
/// Properties:
/// * [authorizationUrl] - The authorization URL.
/// * [pkceRequired] - Indicates if PKCE (RFC 7636) is required.
/// * [refreshUrl] - The URL for obtaining refresh tokens.
/// * [scopes] - Available scopes for the OAuth2 security scheme.
/// * [tokenUrl] - The token URL.
@BuiltValue()
abstract class AuthorizationCodeOAuthFlow implements Built<AuthorizationCodeOAuthFlow, AuthorizationCodeOAuthFlowBuilder> {
  /// The authorization URL.
  @BuiltValueField(wireName: r'authorizationUrl')
  String get authorizationUrl;

  /// Indicates if PKCE (RFC 7636) is required.
  @BuiltValueField(wireName: r'pkceRequired')
  bool? get pkceRequired;

  /// The URL for obtaining refresh tokens.
  @BuiltValueField(wireName: r'refreshUrl')
  String? get refreshUrl;

  /// Available scopes for the OAuth2 security scheme.
  @BuiltValueField(wireName: r'scopes')
  BuiltMap<String, String> get scopes;

  /// The token URL.
  @BuiltValueField(wireName: r'tokenUrl')
  String get tokenUrl;

  AuthorizationCodeOAuthFlow._();

  factory AuthorizationCodeOAuthFlow([void updates(AuthorizationCodeOAuthFlowBuilder b)]) = _$AuthorizationCodeOAuthFlow;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AuthorizationCodeOAuthFlowBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AuthorizationCodeOAuthFlow> get serializer => _$AuthorizationCodeOAuthFlowSerializer();
}

class _$AuthorizationCodeOAuthFlowSerializer implements PrimitiveSerializer<AuthorizationCodeOAuthFlow> {
  @override
  final Iterable<Type> types = const [AuthorizationCodeOAuthFlow, _$AuthorizationCodeOAuthFlow];

  @override
  final String wireName = r'AuthorizationCodeOAuthFlow';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AuthorizationCodeOAuthFlow object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'authorizationUrl';
    yield serializers.serialize(
      object.authorizationUrl,
      specifiedType: const FullType(String),
    );
    if (object.pkceRequired != null) {
      yield r'pkceRequired';
      yield serializers.serialize(
        object.pkceRequired,
        specifiedType: const FullType(bool),
      );
    }
    if (object.refreshUrl != null) {
      yield r'refreshUrl';
      yield serializers.serialize(
        object.refreshUrl,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'scopes';
    yield serializers.serialize(
      object.scopes,
      specifiedType: const FullType(BuiltMap, [FullType(String), FullType(String)]),
    );
    yield r'tokenUrl';
    yield serializers.serialize(
      object.tokenUrl,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AuthorizationCodeOAuthFlow object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AuthorizationCodeOAuthFlowBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'authorizationUrl':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.authorizationUrl = valueDes;
          break;
        case r'pkceRequired':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.pkceRequired = valueDes;
          break;
        case r'refreshUrl':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.refreshUrl = valueDes;
          break;
        case r'scopes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltMap, [FullType(String), FullType(String)]),
          ) as BuiltMap<String, String>;
          result.scopes.replace(valueDes);
          break;
        case r'tokenUrl':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.tokenUrl = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AuthorizationCodeOAuthFlow deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AuthorizationCodeOAuthFlowBuilder();
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

