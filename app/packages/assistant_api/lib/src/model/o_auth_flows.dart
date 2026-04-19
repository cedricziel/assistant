//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/client_credentials_o_auth_flow.dart';
import 'package:assistant_api/src/model/password_o_auth_flow.dart';
import 'package:assistant_api/src/model/device_code_o_auth_flow.dart';
import 'package:assistant_api/src/model/implicit_o_auth_flow.dart';
import 'package:assistant_api/src/model/authorization_code_o_auth_flow.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'o_auth_flows.g.dart';

/// Configuration for supported OAuth 2.0 flows.
///
/// Properties:
/// * [authorizationCode] - Configuration for the OAuth Authorization Code flow.
/// * [clientCredentials] - Configuration for the OAuth Client Credentials flow.
/// * [deviceCode] - Configuration for the OAuth Device Code flow.
/// * [implicit] - Deprecated: Use Authorization Code + PKCE instead.
/// * [password] - Deprecated: Use Authorization Code + PKCE or Device Code.
@BuiltValue()
abstract class OAuthFlows implements Built<OAuthFlows, OAuthFlowsBuilder> {
  /// Configuration for the OAuth Authorization Code flow.
  @BuiltValueField(wireName: r'authorizationCode')
  AuthorizationCodeOAuthFlow? get authorizationCode;

  /// Configuration for the OAuth Client Credentials flow.
  @BuiltValueField(wireName: r'clientCredentials')
  ClientCredentialsOAuthFlow? get clientCredentials;

  /// Configuration for the OAuth Device Code flow.
  @BuiltValueField(wireName: r'deviceCode')
  DeviceCodeOAuthFlow? get deviceCode;

  /// Deprecated: Use Authorization Code + PKCE instead.
  @BuiltValueField(wireName: r'implicit')
  ImplicitOAuthFlow? get implicit;

  /// Deprecated: Use Authorization Code + PKCE or Device Code.
  @BuiltValueField(wireName: r'password')
  PasswordOAuthFlow? get password;

  OAuthFlows._();

  factory OAuthFlows([void updates(OAuthFlowsBuilder b)]) = _$OAuthFlows;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(OAuthFlowsBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<OAuthFlows> get serializer => _$OAuthFlowsSerializer();
}

class _$OAuthFlowsSerializer implements PrimitiveSerializer<OAuthFlows> {
  @override
  final Iterable<Type> types = const [OAuthFlows, _$OAuthFlows];

  @override
  final String wireName = r'OAuthFlows';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    OAuthFlows object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.authorizationCode != null) {
      yield r'authorizationCode';
      yield serializers.serialize(
        object.authorizationCode,
        specifiedType: const FullType.nullable(AuthorizationCodeOAuthFlow),
      );
    }
    if (object.clientCredentials != null) {
      yield r'clientCredentials';
      yield serializers.serialize(
        object.clientCredentials,
        specifiedType: const FullType.nullable(ClientCredentialsOAuthFlow),
      );
    }
    if (object.deviceCode != null) {
      yield r'deviceCode';
      yield serializers.serialize(
        object.deviceCode,
        specifiedType: const FullType.nullable(DeviceCodeOAuthFlow),
      );
    }
    if (object.implicit != null) {
      yield r'implicit';
      yield serializers.serialize(
        object.implicit,
        specifiedType: const FullType.nullable(ImplicitOAuthFlow),
      );
    }
    if (object.password != null) {
      yield r'password';
      yield serializers.serialize(
        object.password,
        specifiedType: const FullType.nullable(PasswordOAuthFlow),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    OAuthFlows object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required OAuthFlowsBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'authorizationCode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(AuthorizationCodeOAuthFlow),
          ) as AuthorizationCodeOAuthFlow?;
          if (valueDes == null) continue;
          result.authorizationCode.replace(valueDes);
          break;
        case r'clientCredentials':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(ClientCredentialsOAuthFlow),
          ) as ClientCredentialsOAuthFlow?;
          if (valueDes == null) continue;
          result.clientCredentials.replace(valueDes);
          break;
        case r'deviceCode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(DeviceCodeOAuthFlow),
          ) as DeviceCodeOAuthFlow?;
          if (valueDes == null) continue;
          result.deviceCode.replace(valueDes);
          break;
        case r'implicit':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(ImplicitOAuthFlow),
          ) as ImplicitOAuthFlow?;
          if (valueDes == null) continue;
          result.implicit.replace(valueDes);
          break;
        case r'password':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(PasswordOAuthFlow),
          ) as PasswordOAuthFlow?;
          if (valueDes == null) continue;
          result.password.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  OAuthFlows deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = OAuthFlowsBuilder();
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

