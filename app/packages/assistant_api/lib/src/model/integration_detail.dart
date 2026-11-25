//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/integration_auth_kind.dart';
import 'package:assistant_api/src/model/integration_layer.dart';
import 'package:assistant_api/src/model/integration_status.dart';
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/space_access_entry.dart';
import 'package:assistant_api/src/model/credential_mode.dart';
import 'package:assistant_api/src/model/integration_trust.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'integration_detail.g.dart';

/// An installed integration with its auth template and space rows.
///
/// Properties:
/// * [authKind]
/// * [catalogEntryId]
/// * [connectUrl]
/// * [createdAt]
/// * [credentialMode]
/// * [enabled]
/// * [hasCredential]
/// * [headerName] - Header that carries the credential, for `header` and `mixed` auth.
/// * [headerValuePrefix] - Text before the secret in the header value, e.g. `Bearer `.
/// * [id] - `int_<uuid>`.
/// * [layer]
/// * [name]
/// * [ownerUserId]
/// * [slug]
/// * [spaceAccess] - Space rows of an org-layer integration, limited to the spaces the caller belongs to unless the caller is an org admin.
/// * [spaceId]
/// * [status]
/// * [trust]
/// * [updatedAt]
@BuiltValue()
abstract class IntegrationDetail
    implements Built<IntegrationDetail, IntegrationDetailBuilder> {
  @BuiltValueField(wireName: r'auth_kind')
  IntegrationAuthKind get authKind;
  // enum authKindEnum {  none,  header,  oauth,  mixed,  };

  @BuiltValueField(wireName: r'catalog_entry_id')
  String? get catalogEntryId;

  @BuiltValueField(wireName: r'connect_url')
  String get connectUrl;

  @BuiltValueField(wireName: r'created_at')
  DateTime get createdAt;

  @BuiltValueField(wireName: r'credential_mode')
  CredentialMode get credentialMode;
  // enum credentialModeEnum {  shared,  per_user,  };

  @BuiltValueField(wireName: r'enabled')
  bool get enabled;

  @BuiltValueField(wireName: r'has_credential')
  bool get hasCredential;

  /// Header that carries the credential, for `header` and `mixed` auth.
  @BuiltValueField(wireName: r'header_name')
  String? get headerName;

  /// Text before the secret in the header value, e.g. `Bearer `.
  @BuiltValueField(wireName: r'header_value_prefix')
  String? get headerValuePrefix;

  /// `int_<uuid>`.
  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'layer')
  IntegrationLayer get layer;
  // enum layerEnum {  org,  space,  user,  };

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'owner_user_id')
  String? get ownerUserId;

  @BuiltValueField(wireName: r'slug')
  String get slug;

  /// Space rows of an org-layer integration, limited to the spaces the caller belongs to unless the caller is an org admin.
  @BuiltValueField(wireName: r'space_access')
  BuiltList<SpaceAccessEntry> get spaceAccess;

  @BuiltValueField(wireName: r'space_id')
  String? get spaceId;

  @BuiltValueField(wireName: r'status')
  IntegrationStatus get status;
  // enum statusEnum {  ready,  needs_connection,  auth_expired,  unreachable,  blocked_by_policy,  disabled,  };

  @BuiltValueField(wireName: r'trust')
  IntegrationTrust get trust;
  // enum trustEnum {  confirm,  trust,  };

  @BuiltValueField(wireName: r'updated_at')
  DateTime get updatedAt;

  IntegrationDetail._();

  factory IntegrationDetail([void updates(IntegrationDetailBuilder b)]) =
      _$IntegrationDetail;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(IntegrationDetailBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<IntegrationDetail> get serializer =>
      _$IntegrationDetailSerializer();
}

class _$IntegrationDetailSerializer
    implements PrimitiveSerializer<IntegrationDetail> {
  @override
  final Iterable<Type> types = const [IntegrationDetail, _$IntegrationDetail];

  @override
  final String wireName = r'IntegrationDetail';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    IntegrationDetail object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'auth_kind';
    yield serializers.serialize(
      object.authKind,
      specifiedType: const FullType(IntegrationAuthKind),
    );
    if (object.catalogEntryId != null) {
      yield r'catalog_entry_id';
      yield serializers.serialize(
        object.catalogEntryId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'connect_url';
    yield serializers.serialize(
      object.connectUrl,
      specifiedType: const FullType(String),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'credential_mode';
    yield serializers.serialize(
      object.credentialMode,
      specifiedType: const FullType(CredentialMode),
    );
    yield r'enabled';
    yield serializers.serialize(
      object.enabled,
      specifiedType: const FullType(bool),
    );
    yield r'has_credential';
    yield serializers.serialize(
      object.hasCredential,
      specifiedType: const FullType(bool),
    );
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
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'layer';
    yield serializers.serialize(
      object.layer,
      specifiedType: const FullType(IntegrationLayer),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    if (object.ownerUserId != null) {
      yield r'owner_user_id';
      yield serializers.serialize(
        object.ownerUserId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'slug';
    yield serializers.serialize(
      object.slug,
      specifiedType: const FullType(String),
    );
    yield r'space_access';
    yield serializers.serialize(
      object.spaceAccess,
      specifiedType: const FullType(BuiltList, [FullType(SpaceAccessEntry)]),
    );
    if (object.spaceId != null) {
      yield r'space_id';
      yield serializers.serialize(
        object.spaceId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(IntegrationStatus),
    );
    yield r'trust';
    yield serializers.serialize(
      object.trust,
      specifiedType: const FullType(IntegrationTrust),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(DateTime),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    IntegrationDetail object, {
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
    required IntegrationDetailBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'auth_kind':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(IntegrationAuthKind),
          ) as IntegrationAuthKind;
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
            specifiedType: const FullType(String),
          ) as String;
          result.connectUrl = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.createdAt = valueDes;
          break;
        case r'credential_mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(CredentialMode),
          ) as CredentialMode;
          result.credentialMode = valueDes;
          break;
        case r'enabled':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.enabled = valueDes;
          break;
        case r'has_credential':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.hasCredential = valueDes;
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
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
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
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'owner_user_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.ownerUserId = valueDes;
          break;
        case r'slug':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.slug = valueDes;
          break;
        case r'space_access':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(BuiltList, [FullType(SpaceAccessEntry)]),
          ) as BuiltList<SpaceAccessEntry>;
          result.spaceAccess.replace(valueDes);
          break;
        case r'space_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.spaceId = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(IntegrationStatus),
          ) as IntegrationStatus;
          result.status = valueDes;
          break;
        case r'trust':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(IntegrationTrust),
          ) as IntegrationTrust;
          result.trust = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
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
  IntegrationDetail deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = IntegrationDetailBuilder();
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
