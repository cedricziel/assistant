//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'interface_instance_response.g.dart';

/// InterfaceInstanceResponse
///
/// Properties:
/// * [config]
/// * [createdAt]
/// * [id]
/// * [interfaceType]
/// * [spaceId]
@BuiltValue()
abstract class InterfaceInstanceResponse
    implements
        Built<InterfaceInstanceResponse, InterfaceInstanceResponseBuilder> {
  @BuiltValueField(wireName: r'config')
  JsonObject? get config;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'interface_type')
  String get interfaceType;

  @BuiltValueField(wireName: r'space_id')
  String get spaceId;

  InterfaceInstanceResponse._();

  factory InterfaceInstanceResponse(
          [void updates(InterfaceInstanceResponseBuilder b)]) =
      _$InterfaceInstanceResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(InterfaceInstanceResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<InterfaceInstanceResponse> get serializer =>
      _$InterfaceInstanceResponseSerializer();
}

class _$InterfaceInstanceResponseSerializer
    implements PrimitiveSerializer<InterfaceInstanceResponse> {
  @override
  final Iterable<Type> types = const [
    InterfaceInstanceResponse,
    _$InterfaceInstanceResponse
  ];

  @override
  final String wireName = r'InterfaceInstanceResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    InterfaceInstanceResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'config';
    yield object.config == null
        ? null
        : serializers.serialize(
            object.config,
            specifiedType: const FullType.nullable(JsonObject),
          );
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
    yield r'interface_type';
    yield serializers.serialize(
      object.interfaceType,
      specifiedType: const FullType(String),
    );
    yield r'space_id';
    yield serializers.serialize(
      object.spaceId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    InterfaceInstanceResponse object, {
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
    required InterfaceInstanceResponseBuilder result,
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
        case r'interface_type':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.interfaceType = valueDes;
          break;
        case r'space_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.spaceId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  InterfaceInstanceResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = InterfaceInstanceResponseBuilder();
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
