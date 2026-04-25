//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_skill_request.g.dart';

/// Body for `POST /api/skills`.
///
/// Properties:
/// * [body]
/// * [description]
/// * [name]
@BuiltValue()
abstract class CreateSkillRequest
    implements Built<CreateSkillRequest, CreateSkillRequestBuilder> {
  @BuiltValueField(wireName: r'body')
  String get body;

  @BuiltValueField(wireName: r'description')
  String get description;

  @BuiltValueField(wireName: r'name')
  String get name;

  CreateSkillRequest._();

  factory CreateSkillRequest([void updates(CreateSkillRequestBuilder b)]) =
      _$CreateSkillRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateSkillRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateSkillRequest> get serializer =>
      _$CreateSkillRequestSerializer();
}

class _$CreateSkillRequestSerializer
    implements PrimitiveSerializer<CreateSkillRequest> {
  @override
  final Iterable<Type> types = const [CreateSkillRequest, _$CreateSkillRequest];

  @override
  final String wireName = r'CreateSkillRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateSkillRequest object, {
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
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateSkillRequest object, {
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
    required CreateSkillRequestBuilder result,
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
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
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
  CreateSkillRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateSkillRequestBuilder();
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
