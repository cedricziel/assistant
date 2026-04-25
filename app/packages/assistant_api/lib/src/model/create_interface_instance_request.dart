//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_interface_instance_request.g.dart';

/// CreateInterfaceInstanceRequest
///
/// Properties:
/// * [config]
/// * [interfaceType] - Interface type: `\"slack\"`, `\"matrix\"`, `\"mattermost\"`, `\"signal\"`, `\"nextcloud\"`.
@BuiltValue()
abstract class CreateInterfaceInstanceRequest
    implements
        Built<CreateInterfaceInstanceRequest,
            CreateInterfaceInstanceRequestBuilder> {
  @BuiltValueField(wireName: r'config')
  JsonObject? get config;

  /// Interface type: `\"slack\"`, `\"matrix\"`, `\"mattermost\"`, `\"signal\"`, `\"nextcloud\"`.
  @BuiltValueField(wireName: r'interface_type')
  String get interfaceType;

  CreateInterfaceInstanceRequest._();

  factory CreateInterfaceInstanceRequest(
          [void updates(CreateInterfaceInstanceRequestBuilder b)]) =
      _$CreateInterfaceInstanceRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateInterfaceInstanceRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateInterfaceInstanceRequest> get serializer =>
      _$CreateInterfaceInstanceRequestSerializer();
}

class _$CreateInterfaceInstanceRequestSerializer
    implements PrimitiveSerializer<CreateInterfaceInstanceRequest> {
  @override
  final Iterable<Type> types = const [
    CreateInterfaceInstanceRequest,
    _$CreateInterfaceInstanceRequest
  ];

  @override
  final String wireName = r'CreateInterfaceInstanceRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateInterfaceInstanceRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.config != null) {
      yield r'config';
      yield serializers.serialize(
        object.config,
        specifiedType: const FullType.nullable(JsonObject),
      );
    }
    yield r'interface_type';
    yield serializers.serialize(
      object.interfaceType,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateInterfaceInstanceRequest object, {
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
    required CreateInterfaceInstanceRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'config':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.config = valueDes;
          break;
        case r'interface_type':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.interfaceType = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CreateInterfaceInstanceRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateInterfaceInstanceRequestBuilder();
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
