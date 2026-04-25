//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_skill_request.g.dart';

/// Body for `PUT /api/skills/{name}`.
///
/// Properties:
/// * [body]
/// * [description]
@BuiltValue()
abstract class UpdateSkillRequest
    implements Built<UpdateSkillRequest, UpdateSkillRequestBuilder> {
  @BuiltValueField(wireName: r'body')
  String get body;

  @BuiltValueField(wireName: r'description')
  String get description;

  UpdateSkillRequest._();

  factory UpdateSkillRequest([void updates(UpdateSkillRequestBuilder b)]) =
      _$UpdateSkillRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateSkillRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateSkillRequest> get serializer =>
      _$UpdateSkillRequestSerializer();
}

class _$UpdateSkillRequestSerializer
    implements PrimitiveSerializer<UpdateSkillRequest> {
  @override
  final Iterable<Type> types = const [UpdateSkillRequest, _$UpdateSkillRequest];

  @override
  final String wireName = r'UpdateSkillRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateSkillRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'body';
    yield serializers.serialize(
      object.body,
      specifiedType: const FullType(String),
    );
    yield r'description';
    yield serializers.serialize(
      object.description,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdateSkillRequest object, {
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
    required UpdateSkillRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'body':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.body = valueDes;
          break;
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.description = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UpdateSkillRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateSkillRequestBuilder();
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
