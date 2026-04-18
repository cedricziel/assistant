//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'register_agent_request.g.dart';

/// Body for `POST /api/agents`.
///
/// Properties:
/// * [card]
/// * [setDefault] - If `true`, make this agent the default after registration.
@BuiltValue()
abstract class RegisterAgentRequest
    implements Built<RegisterAgentRequest, RegisterAgentRequestBuilder> {
  @BuiltValueField(wireName: r'card')
  JsonObject? get card;

  /// If `true`, make this agent the default after registration.
  @BuiltValueField(wireName: r'set_default')
  bool? get setDefault;

  RegisterAgentRequest._();

  factory RegisterAgentRequest([void updates(RegisterAgentRequestBuilder b)]) =
      _$RegisterAgentRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RegisterAgentRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RegisterAgentRequest> get serializer =>
      _$RegisterAgentRequestSerializer();
}

class _$RegisterAgentRequestSerializer
    implements PrimitiveSerializer<RegisterAgentRequest> {
  @override
  final Iterable<Type> types = const [
    RegisterAgentRequest,
    _$RegisterAgentRequest
  ];

  @override
  final String wireName = r'RegisterAgentRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RegisterAgentRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'card';
    yield object.card == null
        ? null
        : serializers.serialize(
            object.card,
            specifiedType: const FullType.nullable(JsonObject),
          );
    if (object.setDefault != null) {
      yield r'set_default';
      yield serializers.serialize(
        object.setDefault,
        specifiedType: const FullType.nullable(bool),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    RegisterAgentRequest object, {
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
    required RegisterAgentRequestBuilder result,
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
        case r'set_default':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(bool),
          ) as bool?;
          if (valueDes == null) continue;
          result.setDefault = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RegisterAgentRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RegisterAgentRequestBuilder();
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
