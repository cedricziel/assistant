//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_interface.g.dart';

/// Declares a combination of a target URL, transport, and protocol version for interacting with the agent.
///
/// Properties:
/// * [protocolBinding] - The protocol binding (e.g., \"JSONRPC\", \"GRPC\", \"HTTP+JSON\").
/// * [protocolVersion] - The version of the A2A protocol this interface exposes (e.g., \"1.0\").
/// * [tenant] - Tenant ID to be used in the request when calling the agent.
/// * [url] - The URL where this interface is available.
@BuiltValue()
abstract class AgentInterface implements Built<AgentInterface, AgentInterfaceBuilder> {
  /// The protocol binding (e.g., \"JSONRPC\", \"GRPC\", \"HTTP+JSON\").
  @BuiltValueField(wireName: r'protocolBinding')
  String get protocolBinding;

  /// The version of the A2A protocol this interface exposes (e.g., \"1.0\").
  @BuiltValueField(wireName: r'protocolVersion')
  String get protocolVersion;

  /// Tenant ID to be used in the request when calling the agent.
  @BuiltValueField(wireName: r'tenant')
  String? get tenant;

  /// The URL where this interface is available.
  @BuiltValueField(wireName: r'url')
  String get url;

  AgentInterface._();

  factory AgentInterface([void updates(AgentInterfaceBuilder b)]) = _$AgentInterface;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentInterfaceBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentInterface> get serializer => _$AgentInterfaceSerializer();
}

class _$AgentInterfaceSerializer implements PrimitiveSerializer<AgentInterface> {
  @override
  final Iterable<Type> types = const [AgentInterface, _$AgentInterface];

  @override
  final String wireName = r'AgentInterface';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentInterface object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'protocolBinding';
    yield serializers.serialize(
      object.protocolBinding,
      specifiedType: const FullType(String),
    );
    yield r'protocolVersion';
    yield serializers.serialize(
      object.protocolVersion,
      specifiedType: const FullType(String),
    );
    if (object.tenant != null) {
      yield r'tenant';
      yield serializers.serialize(
        object.tenant,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'url';
    yield serializers.serialize(
      object.url,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentInterface object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AgentInterfaceBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'protocolBinding':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.protocolBinding = valueDes;
          break;
        case r'protocolVersion':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.protocolVersion = valueDes;
          break;
        case r'tenant':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.tenant = valueDes;
          break;
        case r'url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.url = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AgentInterface deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentInterfaceBuilder();
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

