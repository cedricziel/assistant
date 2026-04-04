//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/agent_skill.dart';
import 'package:assistant_api/src/model/agent_card_signature.dart';
import 'package:assistant_api/src/model/agent_provider.dart';
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/agent_capabilities.dart';
import 'package:assistant_api/src/model/security_requirement.dart';
import 'package:assistant_api/src/model/agent_interface.dart';
import 'package:assistant_api/src/model/security_scheme.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_card.g.dart';

/// A self-describing manifest for an agent.  It provides essential metadata including the agent's identity, capabilities, skills, supported communication methods, and security requirements.
///
/// Properties:
/// * [capabilities] - A2A capability set supported by the agent.
/// * [defaultInputModes] - Input modes the agent supports across all skills (media types).
/// * [defaultOutputModes] - Output media types supported by this agent.
/// * [description] - A human-readable description of the agent's purpose.
/// * [documentationUrl] - A URL providing additional documentation about the agent.
/// * [iconUrl] - A URL to an icon for the agent.
/// * [name] - A human-readable name for the agent.
/// * [provider] - The service provider of the agent.
/// * [securityRequirements] - Security requirements for contacting the agent.
/// * [securitySchemes] - Security scheme definitions for authenticating with this agent.
/// * [signatures] - JSON Web Signatures computed for this `AgentCard`.
/// * [skills] - Skills represent the abilities of an agent.
/// * [supportedInterfaces] - Ordered list of supported interfaces. The first entry is preferred.
/// * [version] - The version of the agent (e.g., \"1.0.0\").
@BuiltValue()
abstract class AgentCard implements Built<AgentCard, AgentCardBuilder> {
  /// A2A capability set supported by the agent.
  @BuiltValueField(wireName: r'capabilities')
  AgentCapabilities get capabilities;

  /// Input modes the agent supports across all skills (media types).
  @BuiltValueField(wireName: r'defaultInputModes')
  BuiltList<String> get defaultInputModes;

  /// Output media types supported by this agent.
  @BuiltValueField(wireName: r'defaultOutputModes')
  BuiltList<String> get defaultOutputModes;

  /// A human-readable description of the agent's purpose.
  @BuiltValueField(wireName: r'description')
  String get description;

  /// A URL providing additional documentation about the agent.
  @BuiltValueField(wireName: r'documentationUrl')
  String? get documentationUrl;

  /// A URL to an icon for the agent.
  @BuiltValueField(wireName: r'iconUrl')
  String? get iconUrl;

  /// A human-readable name for the agent.
  @BuiltValueField(wireName: r'name')
  String get name;

  /// The service provider of the agent.
  @BuiltValueField(wireName: r'provider')
  AgentProvider? get provider;

  /// Security requirements for contacting the agent.
  @BuiltValueField(wireName: r'securityRequirements')
  BuiltList<SecurityRequirement>? get securityRequirements;

  /// Security scheme definitions for authenticating with this agent.
  @BuiltValueField(wireName: r'securitySchemes')
  BuiltMap<String, SecurityScheme>? get securitySchemes;

  /// JSON Web Signatures computed for this `AgentCard`.
  @BuiltValueField(wireName: r'signatures')
  BuiltList<AgentCardSignature>? get signatures;

  /// Skills represent the abilities of an agent.
  @BuiltValueField(wireName: r'skills')
  BuiltList<AgentSkill> get skills;

  /// Ordered list of supported interfaces. The first entry is preferred.
  @BuiltValueField(wireName: r'supportedInterfaces')
  BuiltList<AgentInterface> get supportedInterfaces;

  /// The version of the agent (e.g., \"1.0.0\").
  @BuiltValueField(wireName: r'version')
  String get version;

  AgentCard._();

  factory AgentCard([void updates(AgentCardBuilder b)]) = _$AgentCard;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentCardBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentCard> get serializer => _$AgentCardSerializer();
}

class _$AgentCardSerializer implements PrimitiveSerializer<AgentCard> {
  @override
  final Iterable<Type> types = const [AgentCard, _$AgentCard];

  @override
  final String wireName = r'AgentCard';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentCard object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'capabilities';
    yield serializers.serialize(
      object.capabilities,
      specifiedType: const FullType(AgentCapabilities),
    );
    yield r'defaultInputModes';
    yield serializers.serialize(
      object.defaultInputModes,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'defaultOutputModes';
    yield serializers.serialize(
      object.defaultOutputModes,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'description';
    yield serializers.serialize(
      object.description,
      specifiedType: const FullType(String),
    );
    if (object.documentationUrl != null) {
      yield r'documentationUrl';
      yield serializers.serialize(
        object.documentationUrl,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.iconUrl != null) {
      yield r'iconUrl';
      yield serializers.serialize(
        object.iconUrl,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    if (object.provider != null) {
      yield r'provider';
      yield serializers.serialize(
        object.provider,
        specifiedType: const FullType.nullable(AgentProvider),
      );
    }
    if (object.securityRequirements != null) {
      yield r'securityRequirements';
      yield serializers.serialize(
        object.securityRequirements,
        specifiedType: const FullType(BuiltList, [FullType(SecurityRequirement)]),
      );
    }
    if (object.securitySchemes != null) {
      yield r'securitySchemes';
      yield serializers.serialize(
        object.securitySchemes,
        specifiedType: const FullType(BuiltMap, [FullType(String), FullType(SecurityScheme)]),
      );
    }
    if (object.signatures != null) {
      yield r'signatures';
      yield serializers.serialize(
        object.signatures,
        specifiedType: const FullType(BuiltList, [FullType(AgentCardSignature)]),
      );
    }
    yield r'skills';
    yield serializers.serialize(
      object.skills,
      specifiedType: const FullType(BuiltList, [FullType(AgentSkill)]),
    );
    yield r'supportedInterfaces';
    yield serializers.serialize(
      object.supportedInterfaces,
      specifiedType: const FullType(BuiltList, [FullType(AgentInterface)]),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentCard object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AgentCardBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'capabilities':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(AgentCapabilities),
          ) as AgentCapabilities;
          result.capabilities.replace(valueDes);
          break;
        case r'defaultInputModes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.defaultInputModes.replace(valueDes);
          break;
        case r'defaultOutputModes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.defaultOutputModes.replace(valueDes);
          break;
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.description = valueDes;
          break;
        case r'documentationUrl':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.documentationUrl = valueDes;
          break;
        case r'iconUrl':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.iconUrl = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'provider':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(AgentProvider),
          ) as AgentProvider?;
          if (valueDes == null) continue;
          result.provider.replace(valueDes);
          break;
        case r'securityRequirements':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(SecurityRequirement)]),
          ) as BuiltList<SecurityRequirement>;
          result.securityRequirements.replace(valueDes);
          break;
        case r'securitySchemes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltMap, [FullType(String), FullType(SecurityScheme)]),
          ) as BuiltMap<String, SecurityScheme>;
          result.securitySchemes.replace(valueDes);
          break;
        case r'signatures':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(AgentCardSignature)]),
          ) as BuiltList<AgentCardSignature>;
          result.signatures.replace(valueDes);
          break;
        case r'skills':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(AgentSkill)]),
          ) as BuiltList<AgentSkill>;
          result.skills.replace(valueDes);
          break;
        case r'supportedInterfaces':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(AgentInterface)]),
          ) as BuiltList<AgentInterface>;
          result.supportedInterfaces.replace(valueDes);
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.version = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AgentCard deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentCardBuilder();
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

