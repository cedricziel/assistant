//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'client_info_schema.g.dart';

/// OpenAPI schema mirror for [`assistant_core::auth::ClientInfo`].
///
/// Properties:
/// * [clientId]
/// * [clientName]
/// * [clientSecret]
/// * [grantTypes]
/// * [redirectUris]
/// * [tokenEndpointAuthMethod]
@BuiltValue()
abstract class ClientInfoSchema
    implements Built<ClientInfoSchema, ClientInfoSchemaBuilder> {
  @BuiltValueField(wireName: r'client_id')
  String get clientId;

  @BuiltValueField(wireName: r'client_name')
  String get clientName;

  @BuiltValueField(wireName: r'client_secret')
  String? get clientSecret;

  @BuiltValueField(wireName: r'grant_types')
  BuiltList<String> get grantTypes;

  @BuiltValueField(wireName: r'redirect_uris')
  BuiltList<String> get redirectUris;

  @BuiltValueField(wireName: r'token_endpoint_auth_method')
  String get tokenEndpointAuthMethod;

  ClientInfoSchema._();

  factory ClientInfoSchema([void updates(ClientInfoSchemaBuilder b)]) =
      _$ClientInfoSchema;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ClientInfoSchemaBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ClientInfoSchema> get serializer =>
      _$ClientInfoSchemaSerializer();
}

class _$ClientInfoSchemaSerializer
    implements PrimitiveSerializer<ClientInfoSchema> {
  @override
  final Iterable<Type> types = const [ClientInfoSchema, _$ClientInfoSchema];

  @override
  final String wireName = r'ClientInfoSchema';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ClientInfoSchema object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'client_id';
    yield serializers.serialize(
      object.clientId,
      specifiedType: const FullType(String),
    );
    yield r'client_name';
    yield serializers.serialize(
      object.clientName,
      specifiedType: const FullType(String),
    );
    if (object.clientSecret != null) {
      yield r'client_secret';
      yield serializers.serialize(
        object.clientSecret,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'grant_types';
    yield serializers.serialize(
      object.grantTypes,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'redirect_uris';
    yield serializers.serialize(
      object.redirectUris,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'token_endpoint_auth_method';
    yield serializers.serialize(
      object.tokenEndpointAuthMethod,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ClientInfoSchema object, {
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
    required ClientInfoSchemaBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'client_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.clientId = valueDes;
          break;
        case r'client_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.clientName = valueDes;
          break;
        case r'client_secret':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.clientSecret = valueDes;
          break;
        case r'grant_types':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.grantTypes.replace(valueDes);
          break;
        case r'redirect_uris':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.redirectUris.replace(valueDes);
          break;
        case r'token_endpoint_auth_method':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.tokenEndpointAuthMethod = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ClientInfoSchema deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ClientInfoSchemaBuilder();
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
