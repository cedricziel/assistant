//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/catalog_mode.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_integration_policy_request.g.dart';

/// Body of `PUT /api/integrations/policy`. Replaces the whole policy.
///
/// Properties:
/// * [allowCustomUrls]
/// * [allowUserIntegrations]
/// * [allowedCatalogIds]
/// * [catalogMode]
@BuiltValue()
abstract class UpdateIntegrationPolicyRequest
    implements
        Built<UpdateIntegrationPolicyRequest,
            UpdateIntegrationPolicyRequestBuilder> {
  @BuiltValueField(wireName: r'allow_custom_urls')
  bool get allowCustomUrls;

  @BuiltValueField(wireName: r'allow_user_integrations')
  bool get allowUserIntegrations;

  @BuiltValueField(wireName: r'allowed_catalog_ids')
  BuiltList<String>? get allowedCatalogIds;

  @BuiltValueField(wireName: r'catalog_mode')
  CatalogMode get catalogMode;
  // enum catalogModeEnum {  all,  allowlist,  none,  };

  UpdateIntegrationPolicyRequest._();

  factory UpdateIntegrationPolicyRequest(
          [void updates(UpdateIntegrationPolicyRequestBuilder b)]) =
      _$UpdateIntegrationPolicyRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateIntegrationPolicyRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateIntegrationPolicyRequest> get serializer =>
      _$UpdateIntegrationPolicyRequestSerializer();
}

class _$UpdateIntegrationPolicyRequestSerializer
    implements PrimitiveSerializer<UpdateIntegrationPolicyRequest> {
  @override
  final Iterable<Type> types = const [
    UpdateIntegrationPolicyRequest,
    _$UpdateIntegrationPolicyRequest
  ];

  @override
  final String wireName = r'UpdateIntegrationPolicyRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateIntegrationPolicyRequest object, {
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
    if (object.allowedCatalogIds != null) {
      yield r'allowed_catalog_ids';
      yield serializers.serialize(
        object.allowedCatalogIds,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    yield r'catalog_mode';
    yield serializers.serialize(
      object.catalogMode,
      specifiedType: const FullType(CatalogMode),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdateIntegrationPolicyRequest object, {
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
    required UpdateIntegrationPolicyRequestBuilder result,
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
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UpdateIntegrationPolicyRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateIntegrationPolicyRequestBuilder();
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
