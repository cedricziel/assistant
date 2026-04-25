//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_binding_request.g.dart';

/// CreateBindingRequest
///
/// Properties:
/// * [interfaceInstanceId]
/// * [personaId]
@BuiltValue()
abstract class CreateBindingRequest
    implements Built<CreateBindingRequest, CreateBindingRequestBuilder> {
  @BuiltValueField(wireName: r'interface_instance_id')
  String get interfaceInstanceId;

  @BuiltValueField(wireName: r'persona_id')
  String get personaId;

  CreateBindingRequest._();

  factory CreateBindingRequest([void updates(CreateBindingRequestBuilder b)]) =
      _$CreateBindingRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateBindingRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateBindingRequest> get serializer =>
      _$CreateBindingRequestSerializer();
}

class _$CreateBindingRequestSerializer
    implements PrimitiveSerializer<CreateBindingRequest> {
  @override
  final Iterable<Type> types = const [
    CreateBindingRequest,
    _$CreateBindingRequest
  ];

  @override
  final String wireName = r'CreateBindingRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateBindingRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
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
    CreateBindingRequest object, {
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
    required CreateBindingRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
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
  CreateBindingRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateBindingRequestBuilder();
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
