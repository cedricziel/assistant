//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/catalog_mode.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'integration_policy_response.g.dart';

/// What lower layers may install in this org.
///
/// Properties:
/// * [allowCustomUrls]
/// * [allowUserIntegrations]
/// * [allowedCatalogIds] - Catalog entry ids allowed when `catalog_mode` is `allowlist`.
/// * [catalogMode]
/// * [updatedAt] - Absent until an org admin first saves the policy.
@BuiltValue()
abstract class IntegrationPolicyResponse
    implements
        Built<IntegrationPolicyResponse, IntegrationPolicyResponseBuilder> {
  @BuiltValueField(wireName: r'allow_custom_urls')
  bool get allowCustomUrls;

  @BuiltValueField(wireName: r'allow_user_integrations')
  bool get allowUserIntegrations;

  /// Catalog entry ids allowed when `catalog_mode` is `allowlist`.
  @BuiltValueField(wireName: r'allowed_catalog_ids')
  BuiltList<String> get allowedCatalogIds;

  @BuiltValueField(wireName: r'catalog_mode')
  CatalogMode get catalogMode;
  // enum catalogModeEnum {  all,  allowlist,  none,  };

  /// Absent until an org admin first saves the policy.
  @BuiltValueField(wireName: r'updated_at')
  DateTime? get updatedAt;

  IntegrationPolicyResponse._();

  factory IntegrationPolicyResponse(
          [void updates(IntegrationPolicyResponseBuilder b)]) =
      _$IntegrationPolicyResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(IntegrationPolicyResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<IntegrationPolicyResponse> get serializer =>
      _$IntegrationPolicyResponseSerializer();
}

class _$IntegrationPolicyResponseSerializer
    implements PrimitiveSerializer<IntegrationPolicyResponse> {
  @override
  final Iterable<Type> types = const [
    IntegrationPolicyResponse,
    _$IntegrationPolicyResponse
  ];

  @override
  final String wireName = r'IntegrationPolicyResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    IntegrationPolicyResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'allow_custom_urls';
    yield serializers.serialize(
      object.allowCustomUrls,
      specifiedType: const FullType(bool),
    );
    yield r'allow_user_integrations';
    yield serializers.serialize(
      object.allowUserIntegrations,
      specifiedType: const FullType(bool),
    );
    yield r'allowed_catalog_ids';
    yield serializers.serialize(
      object.allowedCatalogIds,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'catalog_mode';
    yield serializers.serialize(
      object.catalogMode,
      specifiedType: const FullType(CatalogMode),
    );
    if (object.updatedAt != null) {
      yield r'updated_at';
      yield serializers.serialize(
        object.updatedAt,
        specifiedType: const FullType.nullable(DateTime),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    IntegrationPolicyResponse object, {
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
    required IntegrationPolicyResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'allow_custom_urls':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.allowCustomUrls = valueDes;
          break;
        case r'allow_user_integrations':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.allowUserIntegrations = valueDes;
          break;
        case r'allowed_catalog_ids':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.allowedCatalogIds.replace(valueDes);
          break;
        case r'catalog_mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(CatalogMode),
          ) as CatalogMode;
          result.catalogMode = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(DateTime),
          ) as DateTime?;
          if (valueDes == null) continue;
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
  IntegrationPolicyResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = IntegrationPolicyResponseBuilder();
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
