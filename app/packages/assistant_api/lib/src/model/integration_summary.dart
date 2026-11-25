//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/integration_auth_kind.dart';
import 'package:assistant_api/src/model/integration_layer.dart';
import 'package:assistant_api/src/model/integration_status.dart';
import 'package:assistant_api/src/model/credential_mode.dart';
import 'package:assistant_api/src/model/integration_trust.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'integration_summary.g.dart';

/// An installed integration, as listed.
///
/// Properties:
/// * [authKind]
/// * [catalogEntryId] - The catalog entry it was installed from; absent for a custom URL.
/// * [connectUrl]
/// * [createdAt]
/// * [credentialMode]
/// * [enabled]
/// * [hasCredential] - Whether a credential is stored for the caller's subject: the shared credential, or the caller's own for a per-user integration. The value itself is never returned.
/// * [id] - `int_<uuid>`.
/// * [lastConnectedAt] - The last time a connection for the caller's subject succeeded.
/// * [layer]
/// * [name]
/// * [ownerUserId] - The owner of a user-layer integration.
/// * [slug] - Unique within its layer and owner. Part of its tool names.
/// * [spaceId] - The space of a space-layer integration.
/// * [status] - Whether the caller can use it. For an org admin looking at another user's integration, the status is the owner's.
/// * [toolCount] - The number of tools that connection found.
/// * [trust]
/// * [updatedAt]
@BuiltValue()
abstract class IntegrationSummary
    implements Built<IntegrationSummary, IntegrationSummaryBuilder> {
  @BuiltValueField(wireName: r'auth_kind')
  IntegrationAuthKind get authKind;
  // enum authKindEnum {  none,  header,  oauth,  mixed,  };

  /// The catalog entry it was installed from; absent for a custom URL.
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

  /// Whether a credential is stored for the caller's subject: the shared credential, or the caller's own for a per-user integration. The value itself is never returned.
  @BuiltValueField(wireName: r'has_credential')
  bool get hasCredential;

  /// `int_<uuid>`.
  @BuiltValueField(wireName: r'id')
  String get id;

  /// The last time a connection for the caller's subject succeeded.
  @BuiltValueField(wireName: r'last_connected_at')
  DateTime? get lastConnectedAt;

  @BuiltValueField(wireName: r'layer')
  IntegrationLayer get layer;
  // enum layerEnum {  org,  space,  user,  };

  @BuiltValueField(wireName: r'name')
  String get name;

  /// The owner of a user-layer integration.
  @BuiltValueField(wireName: r'owner_user_id')
  String? get ownerUserId;

  /// Unique within its layer and owner. Part of its tool names.
  @BuiltValueField(wireName: r'slug')
  String get slug;

  /// The space of a space-layer integration.
  @BuiltValueField(wireName: r'space_id')
  String? get spaceId;

  /// Whether the caller can use it. For an org admin looking at another user's integration, the status is the owner's.
  @BuiltValueField(wireName: r'status')
  IntegrationStatus get status;
  // enum statusEnum {  ready,  needs_connection,  auth_expired,  unreachable,  blocked_by_policy,  disabled,  };

  /// The number of tools that connection found.
  @BuiltValueField(wireName: r'tool_count')
  int? get toolCount;

  @BuiltValueField(wireName: r'trust')
  IntegrationTrust get trust;
  // enum trustEnum {  confirm,  trust,  };

  @BuiltValueField(wireName: r'updated_at')
  DateTime get updatedAt;

  IntegrationSummary._();

  factory IntegrationSummary([void updates(IntegrationSummaryBuilder b)]) =
      _$IntegrationSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(IntegrationSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<IntegrationSummary> get serializer =>
      _$IntegrationSummarySerializer();
}

class _$IntegrationSummarySerializer
    implements PrimitiveSerializer<IntegrationSummary> {
  @override
  final Iterable<Type> types = const [IntegrationSummary, _$IntegrationSummary];

  @override
  final String wireName = r'IntegrationSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    IntegrationSummary object, {
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
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    if (object.lastConnectedAt != null) {
      yield r'last_connected_at';
      yield serializers.serialize(
        object.lastConnectedAt,
        specifiedType: const FullType.nullable(DateTime),
      );
    }
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
    if (object.toolCount != null) {
      yield r'tool_count';
      yield serializers.serialize(
        object.toolCount,
        specifiedType: const FullType.nullable(int),
      );
    }
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
    IntegrationSummary object, {
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
    required IntegrationSummaryBuilder result,
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
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'last_connected_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(DateTime),
          ) as DateTime?;
          if (valueDes == null) continue;
          result.lastConnectedAt = valueDes;
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
        case r'tool_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.toolCount = valueDes;
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
  IntegrationSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = IntegrationSummaryBuilder();
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
