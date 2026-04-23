//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'client_registration_schema.g.dart';

/// OpenAPI schema mirror for [`assistant_core::auth::ClientRegistration`].
///
/// Properties:
/// * [clientName]
/// * [grantTypes]
/// * [redirectUris]
/// * [responseTypes]
/// * [tokenEndpointAuthMethod]
@BuiltValue()
abstract class ClientRegistrationSchema
    implements
        Built<ClientRegistrationSchema, ClientRegistrationSchemaBuilder> {
  @BuiltValueField(wireName: r'client_name')
  String get clientName;

  @BuiltValueField(wireName: r'grant_types')
  BuiltList<String> get grantTypes;

  @BuiltValueField(wireName: r'redirect_uris')
  BuiltList<String> get redirectUris;

  @BuiltValueField(wireName: r'response_types')
  BuiltList<String>? get responseTypes;

  @BuiltValueField(wireName: r'token_endpoint_auth_method')
  String? get tokenEndpointAuthMethod;

  ClientRegistrationSchema._();

  factory ClientRegistrationSchema(
          [void updates(ClientRegistrationSchemaBuilder b)]) =
      _$ClientRegistrationSchema;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ClientRegistrationSchemaBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ClientRegistrationSchema> get serializer =>
      _$ClientRegistrationSchemaSerializer();
}

class _$ClientRegistrationSchemaSerializer
    implements PrimitiveSerializer<ClientRegistrationSchema> {
  @override
  final Iterable<Type> types = const [
    ClientRegistrationSchema,
    _$ClientRegistrationSchema
  ];

  @override
  final String wireName = r'ClientRegistrationSchema';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ClientRegistrationSchema object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'client_name';
    yield serializers.serialize(
      object.clientName,
      specifiedType: const FullType(String),
    );
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
    if (object.responseTypes != null) {
      yield r'response_types';
      yield serializers.serialize(
        object.responseTypes,
        specifiedType: const FullType.nullable(BuiltList, [FullType(String)]),
      );
    }
    if (object.tokenEndpointAuthMethod != null) {
      yield r'token_endpoint_auth_method';
      yield serializers.serialize(
        object.tokenEndpointAuthMethod,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    ClientRegistrationSchema object, {
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
    required ClientRegistrationSchemaBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'client_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.clientName = valueDes;
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
        case r'response_types':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType.nullable(BuiltList, [FullType(String)]),
          ) as BuiltList<String>?;
          if (valueDes == null) continue;
          result.responseTypes.replace(valueDes);
          break;
        case r'token_endpoint_auth_method':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
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
  ClientRegistrationSchema deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ClientRegistrationSchemaBuilder();
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
