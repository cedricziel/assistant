//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_member_request.g.dart';

/// Body for `PATCH /api/orgs/{org_id}/spaces/{space_id}/members/{user_id}`.
///
/// Properties:
/// * [role] - New role for the member.
@BuiltValue()
abstract class UpdateMemberRequest
    implements Built<UpdateMemberRequest, UpdateMemberRequestBuilder> {
  /// New role for the member.
  @BuiltValueField(wireName: r'role')
  String get role;

  UpdateMemberRequest._();

  factory UpdateMemberRequest([void updates(UpdateMemberRequestBuilder b)]) =
      _$UpdateMemberRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateMemberRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateMemberRequest> get serializer =>
      _$UpdateMemberRequestSerializer();
}

class _$UpdateMemberRequestSerializer
    implements PrimitiveSerializer<UpdateMemberRequest> {
  @override
  final Iterable<Type> types = const [
    UpdateMemberRequest,
    _$UpdateMemberRequest
  ];

  @override
  final String wireName = r'UpdateMemberRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateMemberRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'role';
    yield serializers.serialize(
      object.role,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdateMemberRequest object, {
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
    required UpdateMemberRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'role':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.role = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UpdateMemberRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateMemberRequestBuilder();
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
