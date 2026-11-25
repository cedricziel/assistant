//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/integration_auth_kind.dart';
import 'package:assistant_api/src/model/integration_layer.dart';
import 'package:assistant_api/src/model/credential_mode.dart';
import 'package:assistant_api/src/model/integration_trust.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_integration_request.g.dart';

/// Body of `POST /api/integrations`.  Name exactly one of `catalog_entry_id` and `connect_url`.
///
/// Properties:
/// * [authKind] - Required for a custom URL and for a catalog entry whose auth kind is unknown (choose `none`, `header` or `oauth`). A `mixed` catalog entry may be narrowed to `header` or `oauth`.
/// * [catalogEntryId] - Install this catalog entry. Its URL, auth kind and header template are copied at install time.
/// * [connectUrl] - Install a custom remote MCP server. Needs `name` and `auth_kind`.
/// * [credentialMode] - Defaults to `per_user` for OAuth and `shared` otherwise. User-layer integrations are always `per_user`.
/// * [enabled] - Defaults to `true`.
/// * [headerName] - Header that carries the credential. Required for `header` auth on a custom URL or an unknown catalog entry; rejected when the catalog entry supplies its own template.
/// * [headerValuePrefix] - Text before the secret in the header value. Same rules as `header_name`.
/// * [layer]
/// * [name] - Defaults to the catalog entry's name. Required for a custom URL.
/// * [slug] - Kebab-case, 1–32 characters. Defaults to the catalog entry's slug.
/// * [spaceId] - Required for the space layer, rejected otherwise.
/// * [trust] - Defaults to `confirm`.
@BuiltValue()
abstract class CreateIntegrationRequest
    implements
        Built<CreateIntegrationRequest, CreateIntegrationRequestBuilder> {
  /// Required for a custom URL and for a catalog entry whose auth kind is unknown (choose `none`, `header` or `oauth`). A `mixed` catalog entry may be narrowed to `header` or `oauth`.
  @BuiltValueField(wireName: r'auth_kind')
  IntegrationAuthKind? get authKind;
  // enum authKindEnum {  none,  header,  oauth,  mixed,  };

  /// Install this catalog entry. Its URL, auth kind and header template are copied at install time.
  @BuiltValueField(wireName: r'catalog_entry_id')
  String? get catalogEntryId;

  /// Install a custom remote MCP server. Needs `name` and `auth_kind`.
  @BuiltValueField(wireName: r'connect_url')
  String? get connectUrl;

  /// Defaults to `per_user` for OAuth and `shared` otherwise. User-layer integrations are always `per_user`.
  @BuiltValueField(wireName: r'credential_mode')
  CredentialMode? get credentialMode;
  // enum credentialModeEnum {  shared,  per_user,  };

  /// Defaults to `true`.
  @BuiltValueField(wireName: r'enabled')
  bool? get enabled;

  /// Header that carries the credential. Required for `header` auth on a custom URL or an unknown catalog entry; rejected when the catalog entry supplies its own template.
  @BuiltValueField(wireName: r'header_name')
  String? get headerName;

  /// Text before the secret in the header value. Same rules as `header_name`.
  @BuiltValueField(wireName: r'header_value_prefix')
  String? get headerValuePrefix;

  @BuiltValueField(wireName: r'layer')
  IntegrationLayer get layer;
  // enum layerEnum {  org,  space,  user,  };

  /// Defaults to the catalog entry's name. Required for a custom URL.
  @BuiltValueField(wireName: r'name')
  String? get name;

  /// Kebab-case, 1–32 characters. Defaults to the catalog entry's slug.
  @BuiltValueField(wireName: r'slug')
  String? get slug;

  /// Required for the space layer, rejected otherwise.
  @BuiltValueField(wireName: r'space_id')
  String? get spaceId;

  /// Defaults to `confirm`.
  @BuiltValueField(wireName: r'trust')
  IntegrationTrust? get trust;
  // enum trustEnum {  confirm,  trust,  };

  CreateIntegrationRequest._();

  factory CreateIntegrationRequest(
          [void updates(CreateIntegrationRequestBuilder b)]) =
      _$CreateIntegrationRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateIntegrationRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateIntegrationRequest> get serializer =>
      _$CreateIntegrationRequestSerializer();
}

class _$CreateIntegrationRequestSerializer
    implements PrimitiveSerializer<CreateIntegrationRequest> {
  @override
  final Iterable<Type> types = const [
    CreateIntegrationRequest,
    _$CreateIntegrationRequest
  ];

  @override
  final String wireName = r'CreateIntegrationRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateIntegrationRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.authKind != null) {
      yield r'auth_kind';
      yield serializers.serialize(
        object.authKind,
        specifiedType: const FullType.nullable(IntegrationAuthKind),
      );
    }
    if (object.catalogEntryId != null) {
      yield r'catalog_entry_id';
      yield serializers.serialize(
        object.catalogEntryId,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.connectUrl != null) {
      yield r'connect_url';
      yield serializers.serialize(
        object.connectUrl,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.credentialMode != null) {
      yield r'credential_mode';
      yield serializers.serialize(
        object.credentialMode,
        specifiedType: const FullType.nullable(CredentialMode),
      );
    }
    if (object.enabled != null) {
      yield r'enabled';
      yield serializers.serialize(
        object.enabled,
        specifiedType: const FullType.nullable(bool),
      );
    }
    if (object.headerName != null) {
      yield r'header_name';
      yield serializers.serialize(
        object.headerName,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.headerValuePrefix != null) {
      yield r'header_value_prefix';
      yield serializers.serialize(
        object.headerValuePrefix,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'layer';
    yield serializers.serialize(
      object.layer,
      specifiedType: const FullType(IntegrationLayer),
    );
    if (object.name != null) {
      yield r'name';
      yield serializers.serialize(
        object.name,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.slug != null) {
      yield r'slug';
      yield serializers.serialize(
        object.slug,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.spaceId != null) {
      yield r'space_id';
      yield serializers.serialize(
        object.spaceId,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.trust != null) {
      yield r'trust';
      yield serializers.serialize(
        object.trust,
        specifiedType: const FullType.nullable(IntegrationTrust),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateIntegrationRequest object, {
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
    required CreateIntegrationRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'auth_kind':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(IntegrationAuthKind),
          ) as IntegrationAuthKind?;
          if (valueDes == null) continue;
          result.authKind = valueDes;
          break;
        case r'catalog_entry_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.catalogEntryId = valueDes;
          break;
        case r'connect_url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.connectUrl = valueDes;
          break;
        case r'credential_mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(CredentialMode),
          ) as CredentialMode?;
          if (valueDes == null) continue;
          result.credentialMode = valueDes;
          break;
        case r'enabled':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(bool),
          ) as bool?;
          if (valueDes == null) continue;
          result.enabled = valueDes;
          break;
        case r'header_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.headerName = valueDes;
          break;
        case r'header_value_prefix':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.headerValuePrefix = valueDes;
          break;
        case r'layer':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(IntegrationLayer),
          ) as IntegrationLayer;
          result.layer = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.name = valueDes;
          break;
        case r'slug':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.slug = valueDes;
          break;
        case r'space_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.spaceId = valueDes;
          break;
        case r'trust':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(IntegrationTrust),
          ) as IntegrationTrust?;
          if (valueDes == null) continue;
          result.trust = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CreateIntegrationRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateIntegrationRequestBuilder();
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
