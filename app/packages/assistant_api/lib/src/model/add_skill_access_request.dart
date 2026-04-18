//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'add_skill_access_request.g.dart';

/// Body for `POST /api/personas/{id}/skill-access/skills`.
///
/// Properties:
/// * [skillName]
@BuiltValue()
abstract class AddSkillAccessRequest
    implements Built<AddSkillAccessRequest, AddSkillAccessRequestBuilder> {
  @BuiltValueField(wireName: r'skill_name')
  String get skillName;

  AddSkillAccessRequest._();

  factory AddSkillAccessRequest(
      [void updates(AddSkillAccessRequestBuilder b)]) = _$AddSkillAccessRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AddSkillAccessRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AddSkillAccessRequest> get serializer =>
      _$AddSkillAccessRequestSerializer();
}

class _$AddSkillAccessRequestSerializer
    implements PrimitiveSerializer<AddSkillAccessRequest> {
  @override
  final Iterable<Type> types = const [
    AddSkillAccessRequest,
    _$AddSkillAccessRequest
  ];

  @override
  final String wireName = r'AddSkillAccessRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AddSkillAccessRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'skill_name';
    yield serializers.serialize(
      object.skillName,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AddSkillAccessRequest object, {
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
    required AddSkillAccessRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'skill_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.skillName = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AddSkillAccessRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AddSkillAccessRequestBuilder();
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
