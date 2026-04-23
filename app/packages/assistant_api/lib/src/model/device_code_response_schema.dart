//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'device_code_response_schema.g.dart';

/// OpenAPI schema mirror for [`assistant_auth::oauth2::device::DeviceCodeResponse`].
///
/// Properties:
/// * [deviceCode]
/// * [expiresIn]
/// * [interval]
/// * [userCode]
/// * [verificationUri]
@BuiltValue()
abstract class DeviceCodeResponseSchema
    implements
        Built<DeviceCodeResponseSchema, DeviceCodeResponseSchemaBuilder> {
  @BuiltValueField(wireName: r'device_code')
  String get deviceCode;

  @BuiltValueField(wireName: r'expires_in')
  int get expiresIn;

  @BuiltValueField(wireName: r'interval')
  int get interval;

  @BuiltValueField(wireName: r'user_code')
  String get userCode;

  @BuiltValueField(wireName: r'verification_uri')
  String get verificationUri;

  DeviceCodeResponseSchema._();

  factory DeviceCodeResponseSchema(
          [void updates(DeviceCodeResponseSchemaBuilder b)]) =
      _$DeviceCodeResponseSchema;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeviceCodeResponseSchemaBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeviceCodeResponseSchema> get serializer =>
      _$DeviceCodeResponseSchemaSerializer();
}

class _$DeviceCodeResponseSchemaSerializer
    implements PrimitiveSerializer<DeviceCodeResponseSchema> {
  @override
  final Iterable<Type> types = const [
    DeviceCodeResponseSchema,
    _$DeviceCodeResponseSchema
  ];

  @override
  final String wireName = r'DeviceCodeResponseSchema';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeviceCodeResponseSchema object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'device_code';
    yield serializers.serialize(
      object.deviceCode,
      specifiedType: const FullType(String),
    );
    yield r'expires_in';
    yield serializers.serialize(
      object.expiresIn,
      specifiedType: const FullType(int),
    );
    yield r'interval';
    yield serializers.serialize(
      object.interval,
      specifiedType: const FullType(int),
    );
    yield r'user_code';
    yield serializers.serialize(
      object.userCode,
      specifiedType: const FullType(String),
    );
    yield r'verification_uri';
    yield serializers.serialize(
      object.verificationUri,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    DeviceCodeResponseSchema object, {
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
    required DeviceCodeResponseSchemaBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'device_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.deviceCode = valueDes;
          break;
        case r'expires_in':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.expiresIn = valueDes;
          break;
        case r'interval':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.interval = valueDes;
          break;
        case r'user_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.userCode = valueDes;
          break;
        case r'verification_uri':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.verificationUri = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  DeviceCodeResponseSchema deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeviceCodeResponseSchemaBuilder();
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
