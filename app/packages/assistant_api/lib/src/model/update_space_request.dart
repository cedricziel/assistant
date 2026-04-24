//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_space_request.g.dart';

/// Body for `PATCH /api/orgs/{org_id}/spaces/{id}`.
///
/// Properties:
/// * [name]
@BuiltValue()
abstract class UpdateSpaceRequest
    implements Built<UpdateSpaceRequest, UpdateSpaceRequestBuilder> {
  @BuiltValueField(wireName: r'name')
  String? get name;

  UpdateSpaceRequest._();

  factory UpdateSpaceRequest([void updates(UpdateSpaceRequestBuilder b)]) =
      _$UpdateSpaceRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateSpaceRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateSpaceRequest> get serializer =>
      _$UpdateSpaceRequestSerializer();
}

class _$UpdateSpaceRequestSerializer
    implements PrimitiveSerializer<UpdateSpaceRequest> {
  @override
  final Iterable<Type> types = const [UpdateSpaceRequest, _$UpdateSpaceRequest];

  @override
  final String wireName = r'UpdateSpaceRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateSpaceRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
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
    UpdateSpaceRequest object, {
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
    required UpdateSpaceRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
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
  UpdateSpaceRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateSpaceRequestBuilder();
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
