//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_detail.g.dart';

/// Full agent detail, including the serialized agent card.
///
/// Properties:
/// * [card]
/// * [id]
/// * [isDefault]
@BuiltValue()
abstract class AgentDetail implements Built<AgentDetail, AgentDetailBuilder> {
  @BuiltValueField(wireName: r'card')
  JsonObject? get card;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'is_default')
  bool get isDefault;

  AgentDetail._();

  factory AgentDetail([void updates(AgentDetailBuilder b)]) = _$AgentDetail;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentDetailBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentDetail> get serializer => _$AgentDetailSerializer();
}

class _$AgentDetailSerializer implements PrimitiveSerializer<AgentDetail> {
  @override
  final Iterable<Type> types = const [AgentDetail, _$AgentDetail];

  @override
  final String wireName = r'AgentDetail';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentDetail object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'card';
    yield object.card == null
        ? null
        : serializers.serialize(
            object.card,
            specifiedType: const FullType.nullable(JsonObject),
          );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'is_default';
    yield serializers.serialize(
      object.isDefault,
      specifiedType: const FullType(bool),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentDetail object, {
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
    required AgentDetailBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'card':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.card = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'is_default':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.isDefault = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AgentDetail deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentDetailBuilder();
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
