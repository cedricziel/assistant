//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'persona_skill_access.g.dart';

/// Skill access configuration for a persona.
///
/// Properties:
/// * [mode] 
/// * [skills] 
@BuiltValue()
abstract class PersonaSkillAccess implements Built<PersonaSkillAccess, PersonaSkillAccessBuilder> {
  @BuiltValueField(wireName: r'mode')
  String get mode;

  @BuiltValueField(wireName: r'skills')
  BuiltList<String> get skills;

  PersonaSkillAccess._();

  factory PersonaSkillAccess([void updates(PersonaSkillAccessBuilder b)]) = _$PersonaSkillAccess;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PersonaSkillAccessBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PersonaSkillAccess> get serializer => _$PersonaSkillAccessSerializer();
}

class _$PersonaSkillAccessSerializer implements PrimitiveSerializer<PersonaSkillAccess> {
  @override
  final Iterable<Type> types = const [PersonaSkillAccess, _$PersonaSkillAccess];

  @override
  final String wireName = r'PersonaSkillAccess';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PersonaSkillAccess object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'mode';
    yield serializers.serialize(
      object.mode,
      specifiedType: const FullType(String),
    );
    yield r'skills';
    yield serializers.serialize(
      object.skills,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    PersonaSkillAccess object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required PersonaSkillAccessBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.mode = valueDes;
          break;
        case r'skills':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.skills.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  PersonaSkillAccess deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PersonaSkillAccessBuilder();
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

