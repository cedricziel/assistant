//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_extension.g.dart';

/// A declaration of a protocol extension supported by an Agent.
///
/// Properties:
/// * [description] - A human-readable description of how this agent uses the extension.
/// * [params] - Extension-specific configuration parameters.
/// * [required_] - If true, the client must understand and comply with the extension.
/// * [uri] - The unique URI identifying the extension.
@BuiltValue()
abstract class AgentExtension
    implements Built<AgentExtension, AgentExtensionBuilder> {
  /// A human-readable description of how this agent uses the extension.
  @BuiltValueField(wireName: r'description')
  String? get description;

  /// Extension-specific configuration parameters.
  @BuiltValueField(wireName: r'params')
  BuiltMap<String, JsonObject?>? get params;

  /// If true, the client must understand and comply with the extension.
  @BuiltValueField(wireName: r'required')
  bool? get required_;

  /// The unique URI identifying the extension.
  @BuiltValueField(wireName: r'uri')
  String get uri;

  AgentExtension._();

  factory AgentExtension([void updates(AgentExtensionBuilder b)]) =
      _$AgentExtension;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentExtensionBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentExtension> get serializer =>
      _$AgentExtensionSerializer();
}

class _$AgentExtensionSerializer
    implements PrimitiveSerializer<AgentExtension> {
  @override
  final Iterable<Type> types = const [AgentExtension, _$AgentExtension];

  @override
  final String wireName = r'AgentExtension';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentExtension object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.description != null) {
      yield r'description';
      yield serializers.serialize(
        object.description,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.params != null) {
      yield r'params';
      yield serializers.serialize(
        object.params,
        specifiedType: const FullType.nullable(
            BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
      );
    }
    if (object.required_ != null) {
      yield r'required';
      yield serializers.serialize(
        object.required_,
        specifiedType: const FullType(bool),
      );
    }
    yield r'uri';
    yield serializers.serialize(
      object.uri,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentExtension object, {
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
    required AgentExtensionBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.description = valueDes;
          break;
        case r'params':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(
                BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
          ) as BuiltMap<String, JsonObject?>?;
          if (valueDes == null) continue;
          result.params.replace(valueDes);
          break;
        case r'required':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.required_ = valueDes;
          break;
        case r'uri':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.uri = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AgentExtension deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentExtensionBuilder();
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
