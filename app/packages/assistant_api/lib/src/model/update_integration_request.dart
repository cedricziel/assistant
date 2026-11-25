//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/integration_trust.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_integration_request.g.dart';

/// Body of `PATCH /api/integrations/{id}`. Absent fields stay as they are.  The layer, owner, catalog entry, auth kind and credential mode cannot change: they decide whose credentials exist.
///
/// Properties:
/// * [connectUrl] - Custom URL integrations only. Changing it deletes the stored credentials, so they cannot be sent to the new host.
/// * [enabled]
/// * [headerName] - Custom URL integrations only.
/// * [headerValuePrefix] - Custom URL integrations only.
/// * [name]
/// * [slug]
/// * [trust]
@BuiltValue()
abstract class UpdateIntegrationRequest
    implements
        Built<UpdateIntegrationRequest, UpdateIntegrationRequestBuilder> {
  /// Custom URL integrations only. Changing it deletes the stored credentials, so they cannot be sent to the new host.
  @BuiltValueField(wireName: r'connect_url')
  String? get connectUrl;

  @BuiltValueField(wireName: r'enabled')
  bool? get enabled;

  /// Custom URL integrations only.
  @BuiltValueField(wireName: r'header_name')
  String? get headerName;

  /// Custom URL integrations only.
  @BuiltValueField(wireName: r'header_value_prefix')
  String? get headerValuePrefix;

  @BuiltValueField(wireName: r'name')
  String? get name;

  @BuiltValueField(wireName: r'slug')
  String? get slug;

  @BuiltValueField(wireName: r'trust')
  IntegrationTrust? get trust;
  // enum trustEnum {  confirm,  trust,  };

  UpdateIntegrationRequest._();

  factory UpdateIntegrationRequest(
          [void updates(UpdateIntegrationRequestBuilder b)]) =
      _$UpdateIntegrationRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateIntegrationRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateIntegrationRequest> get serializer =>
      _$UpdateIntegrationRequestSerializer();
}

class _$UpdateIntegrationRequestSerializer
    implements PrimitiveSerializer<UpdateIntegrationRequest> {
  @override
  final Iterable<Type> types = const [
    UpdateIntegrationRequest,
    _$UpdateIntegrationRequest
  ];

  @override
  final String wireName = r'UpdateIntegrationRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateIntegrationRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.connectUrl != null) {
      yield r'connect_url';
      yield serializers.serialize(
        object.connectUrl,
        specifiedType: const FullType.nullable(String),
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
    UpdateIntegrationRequest object, {
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
    required UpdateIntegrationRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'connect_url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.connectUrl = valueDes;
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
  UpdateIntegrationRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateIntegrationRequestBuilder();
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
