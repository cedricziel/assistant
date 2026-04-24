//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_org_request.g.dart';

/// Body for `PATCH /api/orgs/{id}`.
///
/// Properties:
/// * [authMode]
/// * [name]
@BuiltValue()
abstract class UpdateOrgRequest
    implements Built<UpdateOrgRequest, UpdateOrgRequestBuilder> {
  @BuiltValueField(wireName: r'auth_mode')
  String? get authMode;

  @BuiltValueField(wireName: r'name')
  String? get name;

  UpdateOrgRequest._();

  factory UpdateOrgRequest([void updates(UpdateOrgRequestBuilder b)]) =
      _$UpdateOrgRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateOrgRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateOrgRequest> get serializer =>
      _$UpdateOrgRequestSerializer();
}

class _$UpdateOrgRequestSerializer
    implements PrimitiveSerializer<UpdateOrgRequest> {
  @override
  final Iterable<Type> types = const [UpdateOrgRequest, _$UpdateOrgRequest];

  @override
  final String wireName = r'UpdateOrgRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateOrgRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.authMode != null) {
      yield r'auth_mode';
      yield serializers.serialize(
        object.authMode,
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
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdateOrgRequest object, {
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
    required UpdateOrgRequestBuilder result,
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
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.name = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UpdateOrgRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateOrgRequestBuilder();
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
