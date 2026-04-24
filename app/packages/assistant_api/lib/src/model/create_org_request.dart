//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_org_request.g.dart';

/// Body for `POST /api/orgs`.
///
/// Properties:
/// * [authMode] - Auth mode: `\"password\"` or `\"oidc\"`.
/// * [name]
/// * [slug]
@BuiltValue()
abstract class CreateOrgRequest
    implements Built<CreateOrgRequest, CreateOrgRequestBuilder> {
  /// Auth mode: `\"password\"` or `\"oidc\"`.
  @BuiltValueField(wireName: r'auth_mode')
  String? get authMode;

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'slug')
  String get slug;

  CreateOrgRequest._();

  factory CreateOrgRequest([void updates(CreateOrgRequestBuilder b)]) =
      _$CreateOrgRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateOrgRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateOrgRequest> get serializer =>
      _$CreateOrgRequestSerializer();
}

class _$CreateOrgRequestSerializer
    implements PrimitiveSerializer<CreateOrgRequest> {
  @override
  final Iterable<Type> types = const [CreateOrgRequest, _$CreateOrgRequest];

  @override
  final String wireName = r'CreateOrgRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateOrgRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.authMode != null) {
      yield r'auth_mode';
      yield serializers.serialize(
        object.authMode,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'slug';
    yield serializers.serialize(
      object.slug,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateOrgRequest object, {
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
    required CreateOrgRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'auth_mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.authMode = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'slug':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.slug = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CreateOrgRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateOrgRequestBuilder();
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
