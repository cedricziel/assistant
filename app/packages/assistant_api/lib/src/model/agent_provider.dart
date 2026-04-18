//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_provider.g.dart';

/// Represents the service provider of an agent.
///
/// Properties:
/// * [organization] - The name of the provider's organization.
/// * [url] - A URL for the provider's website or documentation.
@BuiltValue()
abstract class AgentProvider
    implements Built<AgentProvider, AgentProviderBuilder> {
  /// The name of the provider's organization.
  @BuiltValueField(wireName: r'organization')
  String get organization;

  /// A URL for the provider's website or documentation.
  @BuiltValueField(wireName: r'url')
  String get url;

  AgentProvider._();

  factory AgentProvider([void updates(AgentProviderBuilder b)]) =
      _$AgentProvider;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentProviderBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentProvider> get serializer =>
      _$AgentProviderSerializer();
}

class _$AgentProviderSerializer implements PrimitiveSerializer<AgentProvider> {
  @override
  final Iterable<Type> types = const [AgentProvider, _$AgentProvider];

  @override
  final String wireName = r'AgentProvider';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentProvider object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'organization';
    yield serializers.serialize(
      object.organization,
      specifiedType: const FullType(String),
    );
    yield r'url';
    yield serializers.serialize(
      object.url,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentProvider object, {
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
    required AgentProviderBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'organization':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.organization = valueDes;
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
  AgentProvider deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentProviderBuilder();
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
