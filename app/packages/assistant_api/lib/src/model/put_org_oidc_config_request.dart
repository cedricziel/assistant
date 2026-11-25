//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'put_org_oidc_config_request.g.dart';

/// Body for `PUT /api/orgs/{id}/oidc`.
///
/// Properties:
/// * [allowedEmailDomains]
/// * [autoProvision] - Defaults to `true`.
/// * [clientId]
/// * [clientSecret] - Omit to keep the stored secret, `null` to clear it.
/// * [issuerUrl] - `https` URL, or `http` for `localhost` / loopback addresses.
@BuiltValue()
abstract class PutOrgOidcConfigRequest
    implements Built<PutOrgOidcConfigRequest, PutOrgOidcConfigRequestBuilder> {
  @BuiltValueField(wireName: r'allowed_email_domains')
  BuiltList<String>? get allowedEmailDomains;

  /// Defaults to `true`.
  @BuiltValueField(wireName: r'auto_provision')
  bool? get autoProvision;

  @BuiltValueField(wireName: r'client_id')
  String get clientId;

  /// Omit to keep the stored secret, `null` to clear it.
  @BuiltValueField(wireName: r'client_secret')
  String? get clientSecret;

  /// `https` URL, or `http` for `localhost` / loopback addresses.
  @BuiltValueField(wireName: r'issuer_url')
  String get issuerUrl;

  PutOrgOidcConfigRequest._();

  factory PutOrgOidcConfigRequest(
          [void updates(PutOrgOidcConfigRequestBuilder b)]) =
      _$PutOrgOidcConfigRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PutOrgOidcConfigRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PutOrgOidcConfigRequest> get serializer =>
      _$PutOrgOidcConfigRequestSerializer();
}

class _$PutOrgOidcConfigRequestSerializer
    implements PrimitiveSerializer<PutOrgOidcConfigRequest> {
  @override
  final Iterable<Type> types = const [
    PutOrgOidcConfigRequest,
    _$PutOrgOidcConfigRequest
  ];

  @override
  final String wireName = r'PutOrgOidcConfigRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PutOrgOidcConfigRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.allowedEmailDomains != null) {
      yield r'allowed_email_domains';
      yield serializers.serialize(
        object.allowedEmailDomains,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    if (object.autoProvision != null) {
      yield r'auto_provision';
      yield serializers.serialize(
        object.autoProvision,
        specifiedType: const FullType(bool),
      );
    }
    yield r'client_id';
    yield serializers.serialize(
      object.clientId,
      specifiedType: const FullType(String),
    );
    if (object.clientSecret != null) {
      yield r'client_secret';
      yield serializers.serialize(
        object.clientSecret,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'issuer_url';
    yield serializers.serialize(
      object.issuerUrl,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    PutOrgOidcConfigRequest object, {
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
    required PutOrgOidcConfigRequestBuilder result,
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
        case r'client_secret':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.clientSecret = valueDes;
          break;
        case r'issuer_url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.issuerUrl = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  PutOrgOidcConfigRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PutOrgOidcConfigRequestBuilder();
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
