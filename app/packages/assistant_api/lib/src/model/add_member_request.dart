//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'add_member_request.g.dart';

/// Body for `POST /api/orgs/{org_id}/spaces/{space_id}/members`.
///
/// Properties:
/// * [role] - Role: `\"org-admin\"`, `\"space-admin\"`, `\"member\"`, or `\"viewer\"`.
/// * [userId]
@BuiltValue()
abstract class AddMemberRequest
    implements Built<AddMemberRequest, AddMemberRequestBuilder> {
  /// Role: `\"org-admin\"`, `\"space-admin\"`, `\"member\"`, or `\"viewer\"`.
  @BuiltValueField(wireName: r'role')
  String get role;

  @BuiltValueField(wireName: r'user_id')
  String get userId;

  AddMemberRequest._();

  factory AddMemberRequest([void updates(AddMemberRequestBuilder b)]) =
      _$AddMemberRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AddMemberRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AddMemberRequest> get serializer =>
      _$AddMemberRequestSerializer();
}

class _$AddMemberRequestSerializer
    implements PrimitiveSerializer<AddMemberRequest> {
  @override
  final Iterable<Type> types = const [AddMemberRequest, _$AddMemberRequest];

  @override
  final String wireName = r'AddMemberRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AddMemberRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'role';
    yield serializers.serialize(
      object.role,
      specifiedType: const FullType(String),
    );
    yield r'user_id';
    yield serializers.serialize(
      object.userId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AddMemberRequest object, {
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
    required AddMemberRequestBuilder result,
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
        case r'user_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.userId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AddMemberRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AddMemberRequestBuilder();
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
