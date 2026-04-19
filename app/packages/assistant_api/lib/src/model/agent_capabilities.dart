//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/agent_extension.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_capabilities.g.dart';

/// Defines optional capabilities supported by an agent.
///
/// Properties:
/// * [extendedAgentCard] - Indicates if the agent supports providing an extended agent card.
/// * [extensions] - Protocol extensions supported by the agent.
/// * [pushNotifications] - Indicates if the agent supports push notifications.
/// * [streaming] - Indicates if the agent supports streaming responses.
@BuiltValue()
abstract class AgentCapabilities implements Built<AgentCapabilities, AgentCapabilitiesBuilder> {
  /// Indicates if the agent supports providing an extended agent card.
  @BuiltValueField(wireName: r'extendedAgentCard')
  bool? get extendedAgentCard;

  /// Protocol extensions supported by the agent.
  @BuiltValueField(wireName: r'extensions')
  BuiltList<AgentExtension>? get extensions;

  /// Indicates if the agent supports push notifications.
  @BuiltValueField(wireName: r'pushNotifications')
  bool? get pushNotifications;

  /// Indicates if the agent supports streaming responses.
  @BuiltValueField(wireName: r'streaming')
  bool? get streaming;

  AgentCapabilities._();

  factory AgentCapabilities([void updates(AgentCapabilitiesBuilder b)]) = _$AgentCapabilities;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentCapabilitiesBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentCapabilities> get serializer => _$AgentCapabilitiesSerializer();
}

class _$AgentCapabilitiesSerializer implements PrimitiveSerializer<AgentCapabilities> {
  @override
  final Iterable<Type> types = const [AgentCapabilities, _$AgentCapabilities];

  @override
  final String wireName = r'AgentCapabilities';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentCapabilities object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.extendedAgentCard != null) {
      yield r'extendedAgentCard';
      yield serializers.serialize(
        object.extendedAgentCard,
        specifiedType: const FullType.nullable(bool),
      );
    }
    if (object.extensions != null) {
      yield r'extensions';
      yield serializers.serialize(
        object.extensions,
        specifiedType: const FullType(BuiltList, [FullType(AgentExtension)]),
      );
    }
    if (object.pushNotifications != null) {
      yield r'pushNotifications';
      yield serializers.serialize(
        object.pushNotifications,
        specifiedType: const FullType.nullable(bool),
      );
    }
    if (object.streaming != null) {
      yield r'streaming';
      yield serializers.serialize(
        object.streaming,
        specifiedType: const FullType.nullable(bool),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentCapabilities object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AgentCapabilitiesBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'extendedAgentCard':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(bool),
          ) as bool?;
          if (valueDes == null) continue;
          result.extendedAgentCard = valueDes;
          break;
        case r'extensions':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(AgentExtension)]),
          ) as BuiltList<AgentExtension>;
          result.extensions.replace(valueDes);
          break;
        case r'pushNotifications':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(bool),
          ) as bool?;
          if (valueDes == null) continue;
          result.pushNotifications = valueDes;
          break;
        case r'streaming':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(bool),
          ) as bool?;
          if (valueDes == null) continue;
          result.streaming = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AgentCapabilities deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentCapabilitiesBuilder();
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

