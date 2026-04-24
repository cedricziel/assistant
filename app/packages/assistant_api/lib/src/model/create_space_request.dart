//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_space_request.g.dart';

/// Body for `POST /api/orgs/{org_id}/spaces`.
///
/// Properties:
/// * [name]
/// * [slug]
@BuiltValue()
abstract class CreateSpaceRequest
    implements Built<CreateSpaceRequest, CreateSpaceRequestBuilder> {
  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'slug')
  String get slug;

  CreateSpaceRequest._();

  factory CreateSpaceRequest([void updates(CreateSpaceRequestBuilder b)]) =
      _$CreateSpaceRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateSpaceRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateSpaceRequest> get serializer =>
      _$CreateSpaceRequestSerializer();
}

class _$CreateSpaceRequestSerializer
    implements PrimitiveSerializer<CreateSpaceRequest> {
  @override
  final Iterable<Type> types = const [CreateSpaceRequest, _$CreateSpaceRequest];

  @override
  final String wireName = r'CreateSpaceRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateSpaceRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
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
    CreateSpaceRequest object, {
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
    required CreateSpaceRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
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
  CreateSpaceRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateSpaceRequestBuilder();
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
