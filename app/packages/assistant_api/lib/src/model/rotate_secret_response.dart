//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'rotate_secret_response.g.dart';

/// Response after rotating a webhook secret.
///
/// Properties:
/// * [secret]
@BuiltValue()
abstract class RotateSecretResponse
    implements Built<RotateSecretResponse, RotateSecretResponseBuilder> {
  @BuiltValueField(wireName: r'secret')
  String get secret;

  RotateSecretResponse._();

  factory RotateSecretResponse([void updates(RotateSecretResponseBuilder b)]) =
      _$RotateSecretResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RotateSecretResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RotateSecretResponse> get serializer =>
      _$RotateSecretResponseSerializer();
}

class _$RotateSecretResponseSerializer
    implements PrimitiveSerializer<RotateSecretResponse> {
  @override
  final Iterable<Type> types = const [
    RotateSecretResponse,
    _$RotateSecretResponse
  ];

  @override
  final String wireName = r'RotateSecretResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RotateSecretResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'secret';
    yield serializers.serialize(
      object.secret,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RotateSecretResponse object, {
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
    required RotateSecretResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'secret':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.secret = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RotateSecretResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RotateSecretResponseBuilder();
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
