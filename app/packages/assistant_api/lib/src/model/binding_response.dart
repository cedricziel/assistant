//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'binding_response.g.dart';

/// BindingResponse
///
/// Properties:
/// * [createdAt]
/// * [id]
/// * [interfaceInstanceId]
/// * [personaId]
@BuiltValue()
abstract class BindingResponse
    implements Built<BindingResponse, BindingResponseBuilder> {
  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'interface_instance_id')
  String get interfaceInstanceId;

  @BuiltValueField(wireName: r'persona_id')
  String get personaId;

  BindingResponse._();

  factory BindingResponse([void updates(BindingResponseBuilder b)]) =
      _$BindingResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(BindingResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<BindingResponse> get serializer =>
      _$BindingResponseSerializer();
}

class _$BindingResponseSerializer
    implements PrimitiveSerializer<BindingResponse> {
  @override
  final Iterable<Type> types = const [BindingResponse, _$BindingResponse];

  @override
  final String wireName = r'BindingResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    BindingResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'interface_instance_id';
    yield serializers.serialize(
      object.interfaceInstanceId,
      specifiedType: const FullType(String),
    );
    yield r'persona_id';
    yield serializers.serialize(
      object.personaId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    BindingResponse object, {
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
    required BindingResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'interface_instance_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.interfaceInstanceId = valueDes;
          break;
        case r'persona_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.personaId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  BindingResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = BindingResponseBuilder();
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
