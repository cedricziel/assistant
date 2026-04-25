//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'vapid_key_response.g.dart';

/// VAPID public key returned to the client.
///
/// Properties:
/// * [publicKey] - Base64url-encoded uncompressed P-256 public key (65 bytes). Pass this as `applicationServerKey` to `PushManager.subscribe()`.
@BuiltValue()
abstract class VapidKeyResponse
    implements Built<VapidKeyResponse, VapidKeyResponseBuilder> {
  /// Base64url-encoded uncompressed P-256 public key (65 bytes). Pass this as `applicationServerKey` to `PushManager.subscribe()`.
  @BuiltValueField(wireName: r'public_key')
  String get publicKey;

  VapidKeyResponse._();

  factory VapidKeyResponse([void updates(VapidKeyResponseBuilder b)]) =
      _$VapidKeyResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(VapidKeyResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<VapidKeyResponse> get serializer =>
      _$VapidKeyResponseSerializer();
}

class _$VapidKeyResponseSerializer
    implements PrimitiveSerializer<VapidKeyResponse> {
  @override
  final Iterable<Type> types = const [VapidKeyResponse, _$VapidKeyResponse];

  @override
  final String wireName = r'VapidKeyResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    VapidKeyResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'public_key';
    yield serializers.serialize(
      object.publicKey,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    VapidKeyResponse object, {
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
    required VapidKeyResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'public_key':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.publicKey = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  VapidKeyResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = VapidKeyResponseBuilder();
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
