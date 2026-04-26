//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/user_detail.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_current_user_response.g.dart';

/// Response for `PATCH /api/users/me`. Contains the (possibly updated) `UserDetail` plus a `previous_email` field present only when the email actually changed.
///
/// Properties:
/// * [previousEmail]
/// * [user]
@BuiltValue()
abstract class UpdateCurrentUserResponse
    implements
        Built<UpdateCurrentUserResponse, UpdateCurrentUserResponseBuilder> {
  @BuiltValueField(wireName: r'previous_email')
  String? get previousEmail;

  @BuiltValueField(wireName: r'user')
  UserDetail get user;

  UpdateCurrentUserResponse._();

  factory UpdateCurrentUserResponse(
          [void updates(UpdateCurrentUserResponseBuilder b)]) =
      _$UpdateCurrentUserResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateCurrentUserResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateCurrentUserResponse> get serializer =>
      _$UpdateCurrentUserResponseSerializer();
}

class _$UpdateCurrentUserResponseSerializer
    implements PrimitiveSerializer<UpdateCurrentUserResponse> {
  @override
  final Iterable<Type> types = const [
    UpdateCurrentUserResponse,
    _$UpdateCurrentUserResponse
  ];

  @override
  final String wireName = r'UpdateCurrentUserResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateCurrentUserResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.previousEmail != null) {
      yield r'previous_email';
      yield serializers.serialize(
        object.previousEmail,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'user';
    yield serializers.serialize(
      object.user,
      specifiedType: const FullType(UserDetail),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdateCurrentUserResponse object, {
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
    required UpdateCurrentUserResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'previous_email':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.previousEmail = valueDes;
          break;
        case r'user':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(UserDetail),
          ) as UserDetail;
          result.user.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UpdateCurrentUserResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateCurrentUserResponseBuilder();
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
