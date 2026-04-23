//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'server_metadata.g.dart';

/// RFC 8414 authorization server metadata.
///
/// Properties:
/// * [authorizationEndpoint]
/// * [codeChallengeMethodsSupported]
/// * [deviceAuthorizationEndpoint]
/// * [grantTypesSupported]
/// * [issuer]
/// * [registrationEndpoint]
/// * [responseTypesSupported]
/// * [revocationEndpoint]
/// * [tokenEndpoint]
/// * [tokenEndpointAuthMethodsSupported]
@BuiltValue()
abstract class ServerMetadata
    implements Built<ServerMetadata, ServerMetadataBuilder> {
  @BuiltValueField(wireName: r'authorization_endpoint')
  String get authorizationEndpoint;

  @BuiltValueField(wireName: r'code_challenge_methods_supported')
  BuiltList<String> get codeChallengeMethodsSupported;

  @BuiltValueField(wireName: r'device_authorization_endpoint')
  String get deviceAuthorizationEndpoint;

  @BuiltValueField(wireName: r'grant_types_supported')
  BuiltList<String> get grantTypesSupported;

  @BuiltValueField(wireName: r'issuer')
  String get issuer;

  @BuiltValueField(wireName: r'registration_endpoint')
  String get registrationEndpoint;

  @BuiltValueField(wireName: r'response_types_supported')
  BuiltList<String> get responseTypesSupported;

  @BuiltValueField(wireName: r'revocation_endpoint')
  String get revocationEndpoint;

  @BuiltValueField(wireName: r'token_endpoint')
  String get tokenEndpoint;

  @BuiltValueField(wireName: r'token_endpoint_auth_methods_supported')
  BuiltList<String> get tokenEndpointAuthMethodsSupported;

  ServerMetadata._();

  factory ServerMetadata([void updates(ServerMetadataBuilder b)]) =
      _$ServerMetadata;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ServerMetadataBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ServerMetadata> get serializer =>
      _$ServerMetadataSerializer();
}

class _$ServerMetadataSerializer
    implements PrimitiveSerializer<ServerMetadata> {
  @override
  final Iterable<Type> types = const [ServerMetadata, _$ServerMetadata];

  @override
  final String wireName = r'ServerMetadata';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ServerMetadata object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'authorization_endpoint';
    yield serializers.serialize(
      object.authorizationEndpoint,
      specifiedType: const FullType(String),
    );
    yield r'code_challenge_methods_supported';
    yield serializers.serialize(
      object.codeChallengeMethodsSupported,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'device_authorization_endpoint';
    yield serializers.serialize(
      object.deviceAuthorizationEndpoint,
      specifiedType: const FullType(String),
    );
    yield r'grant_types_supported';
    yield serializers.serialize(
      object.grantTypesSupported,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'issuer';
    yield serializers.serialize(
      object.issuer,
      specifiedType: const FullType(String),
    );
    yield r'registration_endpoint';
    yield serializers.serialize(
      object.registrationEndpoint,
      specifiedType: const FullType(String),
    );
    yield r'response_types_supported';
    yield serializers.serialize(
      object.responseTypesSupported,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'revocation_endpoint';
    yield serializers.serialize(
      object.revocationEndpoint,
      specifiedType: const FullType(String),
    );
    yield r'token_endpoint';
    yield serializers.serialize(
      object.tokenEndpoint,
      specifiedType: const FullType(String),
    );
    yield r'token_endpoint_auth_methods_supported';
    yield serializers.serialize(
      object.tokenEndpointAuthMethodsSupported,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ServerMetadata object, {
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
    required ServerMetadataBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'authorization_endpoint':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.authorizationEndpoint = valueDes;
          break;
        case r'code_challenge_methods_supported':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.codeChallengeMethodsSupported.replace(valueDes);
          break;
        case r'device_authorization_endpoint':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.deviceAuthorizationEndpoint = valueDes;
          break;
        case r'grant_types_supported':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.grantTypesSupported.replace(valueDes);
          break;
        case r'issuer':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.issuer = valueDes;
          break;
        case r'registration_endpoint':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.registrationEndpoint = valueDes;
          break;
        case r'response_types_supported':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.responseTypesSupported.replace(valueDes);
          break;
        case r'revocation_endpoint':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.revocationEndpoint = valueDes;
          break;
        case r'token_endpoint':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.tokenEndpoint = valueDes;
          break;
        case r'token_endpoint_auth_methods_supported':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.tokenEndpointAuthMethodsSupported.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ServerMetadata deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ServerMetadataBuilder();
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
