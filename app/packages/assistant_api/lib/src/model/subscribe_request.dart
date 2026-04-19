//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'subscribe_request.g.dart';

/// Browser push subscription key material.
///
/// Properties:
/// * [auth] - Base64url-encoded 16-byte authentication secret from the browser subscription.
/// * [endpoint] - The push endpoint URL assigned by the browser's push service.
/// * [p256dh] - Base64url-encoded P-256 ECDH public key from the browser subscription.
@BuiltValue()
abstract class SubscribeRequest implements Built<SubscribeRequest, SubscribeRequestBuilder> {
  /// Base64url-encoded 16-byte authentication secret from the browser subscription.
  @BuiltValueField(wireName: r'auth')
  String get auth;

  /// The push endpoint URL assigned by the browser's push service.
  @BuiltValueField(wireName: r'endpoint')
  String get endpoint;

  /// Base64url-encoded P-256 ECDH public key from the browser subscription.
  @BuiltValueField(wireName: r'p256dh')
  String get p256dh;

  SubscribeRequest._();

  factory SubscribeRequest([void updates(SubscribeRequestBuilder b)]) = _$SubscribeRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SubscribeRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SubscribeRequest> get serializer => _$SubscribeRequestSerializer();
}

class _$SubscribeRequestSerializer implements PrimitiveSerializer<SubscribeRequest> {
  @override
  final Iterable<Type> types = const [SubscribeRequest, _$SubscribeRequest];

  @override
  final String wireName = r'SubscribeRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SubscribeRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'auth';
    yield serializers.serialize(
      object.auth,
      specifiedType: const FullType(String),
    );
    yield r'endpoint';
    yield serializers.serialize(
      object.endpoint,
      specifiedType: const FullType(String),
    );
    yield r'p256dh';
    yield serializers.serialize(
      object.p256dh,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SubscribeRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SubscribeRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'auth':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.auth = valueDes;
          break;
        case r'endpoint':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.endpoint = valueDes;
          break;
        case r'p256dh':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.p256dh = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SubscribeRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SubscribeRequestBuilder();
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

