//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/security_requirement.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_skill.g.dart';

/// Represents a distinct capability or function that an agent can perform.
///
/// Properties:
/// * [description] - A detailed description of the skill.
/// * [examples] - Example prompts or scenarios that this skill can handle.
/// * [id] - A unique identifier for the skill.
/// * [inputModes] - Supported input media types, overriding agent defaults.
/// * [name] - A human-readable name for the skill.
/// * [outputModes] - Supported output media types, overriding agent defaults.
/// * [securityRequirements] - Security schemes necessary for this skill.
/// * [tags] - Keywords describing the skill's capabilities.
@BuiltValue()
abstract class AgentSkill implements Built<AgentSkill, AgentSkillBuilder> {
  /// A detailed description of the skill.
  @BuiltValueField(wireName: r'description')
  String get description;

  /// Example prompts or scenarios that this skill can handle.
  @BuiltValueField(wireName: r'examples')
  BuiltList<String>? get examples;

  /// A unique identifier for the skill.
  @BuiltValueField(wireName: r'id')
  String get id;

  /// Supported input media types, overriding agent defaults.
  @BuiltValueField(wireName: r'inputModes')
  BuiltList<String>? get inputModes;

  /// A human-readable name for the skill.
  @BuiltValueField(wireName: r'name')
  String get name;

  /// Supported output media types, overriding agent defaults.
  @BuiltValueField(wireName: r'outputModes')
  BuiltList<String>? get outputModes;

  /// Security schemes necessary for this skill.
  @BuiltValueField(wireName: r'securityRequirements')
  BuiltList<SecurityRequirement>? get securityRequirements;

  /// Keywords describing the skill's capabilities.
  @BuiltValueField(wireName: r'tags')
  BuiltList<String> get tags;

  AgentSkill._();

  factory AgentSkill([void updates(AgentSkillBuilder b)]) = _$AgentSkill;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentSkillBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentSkill> get serializer => _$AgentSkillSerializer();
}

class _$AgentSkillSerializer implements PrimitiveSerializer<AgentSkill> {
  @override
  final Iterable<Type> types = const [AgentSkill, _$AgentSkill];

  @override
  final String wireName = r'AgentSkill';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentSkill object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'description';
    yield serializers.serialize(
      object.description,
      specifiedType: const FullType(String),
    );
    if (object.examples != null) {
      yield r'examples';
      yield serializers.serialize(
        object.examples,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    if (object.inputModes != null) {
      yield r'inputModes';
      yield serializers.serialize(
        object.inputModes,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    if (object.outputModes != null) {
      yield r'outputModes';
      yield serializers.serialize(
        object.outputModes,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    if (object.securityRequirements != null) {
      yield r'securityRequirements';
      yield serializers.serialize(
        object.securityRequirements,
        specifiedType: const FullType(BuiltList, [FullType(SecurityRequirement)]),
      );
    }
    yield r'tags';
    yield serializers.serialize(
      object.tags,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentSkill object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AgentSkillBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.description = valueDes;
          break;
        case r'examples':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.examples.replace(valueDes);
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'inputModes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.inputModes.replace(valueDes);
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'outputModes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.outputModes.replace(valueDes);
          break;
        case r'securityRequirements':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(SecurityRequirement)]),
          ) as BuiltList<SecurityRequirement>;
          result.securityRequirements.replace(valueDes);
          break;
        case r'tags':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.tags.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AgentSkill deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentSkillBuilder();
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

