//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_current_user_request.g.dart';

/// Body for `PATCH /api/users/me`. All fields optional; an empty body is a no-op that returns the current `UserDetail`.
///
/// Properties:
/// * [email]
/// * [name]
@BuiltValue()
abstract class UpdateCurrentUserRequest
    implements
        Built<UpdateCurrentUserRequest, UpdateCurrentUserRequestBuilder> {
  @BuiltValueField(wireName: r'email')
  String? get email;

  @BuiltValueField(wireName: r'name')
  String? get name;

  UpdateCurrentUserRequest._();

  factory UpdateCurrentUserRequest(
          [void updates(UpdateCurrentUserRequestBuilder b)]) =
      _$UpdateCurrentUserRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateCurrentUserRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateCurrentUserRequest> get serializer =>
      _$UpdateCurrentUserRequestSerializer();
}

class _$UpdateCurrentUserRequestSerializer
    implements PrimitiveSerializer<UpdateCurrentUserRequest> {
  @override
  final Iterable<Type> types = const [
    UpdateCurrentUserRequest,
    _$UpdateCurrentUserRequest
  ];

  @override
  final String wireName = r'UpdateCurrentUserRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateCurrentUserRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.email != null) {
      yield r'email';
      yield serializers.serialize(
        object.email,
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
    UpdateCurrentUserRequest object, {
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
    required UpdateCurrentUserRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'email':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.email = valueDes;
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
  UpdateCurrentUserRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateCurrentUserRequestBuilder();
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
