//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'org_oidc_config_response.g.dart';

/// An org's OIDC settings as returned by the API. The client secret is never included.
///
/// Properties:
/// * [allowedEmailDomains] - Email domains allowed to sign in; empty allows any domain.
/// * [autoProvision]
/// * [clientId]
/// * [createdAt]
/// * [hasClientSecret] - Whether a client secret is stored.
/// * [issuerUrl]
/// * [updatedAt]
@BuiltValue()
abstract class OrgOidcConfigResponse
    implements Built<OrgOidcConfigResponse, OrgOidcConfigResponseBuilder> {
  /// Email domains allowed to sign in; empty allows any domain.
  @BuiltValueField(wireName: r'allowed_email_domains')
  BuiltList<String> get allowedEmailDomains;

  @BuiltValueField(wireName: r'auto_provision')
  bool get autoProvision;

  @BuiltValueField(wireName: r'client_id')
  String get clientId;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  /// Whether a client secret is stored.
  @BuiltValueField(wireName: r'has_client_secret')
  bool get hasClientSecret;

  @BuiltValueField(wireName: r'issuer_url')
  String get issuerUrl;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  OrgOidcConfigResponse._();

  factory OrgOidcConfigResponse(
      [void updates(OrgOidcConfigResponseBuilder b)]) = _$OrgOidcConfigResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(OrgOidcConfigResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<OrgOidcConfigResponse> get serializer =>
      _$OrgOidcConfigResponseSerializer();
}

class _$OrgOidcConfigResponseSerializer
    implements PrimitiveSerializer<OrgOidcConfigResponse> {
  @override
  final Iterable<Type> types = const [
    OrgOidcConfigResponse,
    _$OrgOidcConfigResponse
  ];

  @override
  final String wireName = r'OrgOidcConfigResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    OrgOidcConfigResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'allowed_email_domains';
    yield serializers.serialize(
      object.allowedEmailDomains,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'auto_provision';
    yield serializers.serialize(
      object.autoProvision,
      specifiedType: const FullType(bool),
    );
    yield r'client_id';
    yield serializers.serialize(
      object.clientId,
      specifiedType: const FullType(String),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    yield r'has_client_secret';
    yield serializers.serialize(
      object.hasClientSecret,
      specifiedType: const FullType(bool),
    );
    yield r'issuer_url';
    yield serializers.serialize(
      object.issuerUrl,
      specifiedType: const FullType(String),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    OrgOidcConfigResponse object, {
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
    required OrgOidcConfigResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'allowed_email_domains':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.allowedEmailDomains.replace(valueDes);
          break;
        case r'auto_provision':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.autoProvision = valueDes;
          break;
        case r'client_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.clientId = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'has_client_secret':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.hasClientSecret = valueDes;
          break;
        case r'issuer_url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.issuerUrl = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  OrgOidcConfigResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = OrgOidcConfigResponseBuilder();
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
