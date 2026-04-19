//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_agent_request.g.dart';

/// Body for `PUT /api/agents/{id}`.
///
/// Properties:
/// * [card] 
@BuiltValue()
abstract class UpdateAgentRequest implements Built<UpdateAgentRequest, UpdateAgentRequestBuilder> {
  @BuiltValueField(wireName: r'card')
  JsonObject? get card;

  UpdateAgentRequest._();

  factory UpdateAgentRequest([void updates(UpdateAgentRequestBuilder b)]) = _$UpdateAgentRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateAgentRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateAgentRequest> get serializer => _$UpdateAgentRequestSerializer();
}

class _$UpdateAgentRequestSerializer implements PrimitiveSerializer<UpdateAgentRequest> {
  @override
  final Iterable<Type> types = const [UpdateAgentRequest, _$UpdateAgentRequest];

  @override
  final String wireName = r'UpdateAgentRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateAgentRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'card';
    yield object.card == null ? null : serializers.serialize(
      object.card,
      specifiedType: const FullType.nullable(JsonObject),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdateAgentRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required UpdateAgentRequestBuilder result,
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
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UpdateAgentRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateAgentRequestBuilder();
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

