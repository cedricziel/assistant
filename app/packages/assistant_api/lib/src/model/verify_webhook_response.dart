//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'verify_webhook_response.g.dart';

/// Response after attempting webhook verification.
///
/// Properties:
/// * [detail] 
/// * [success] 
@BuiltValue()
abstract class VerifyWebhookResponse implements Built<VerifyWebhookResponse, VerifyWebhookResponseBuilder> {
  @BuiltValueField(wireName: r'detail')
  String get detail;

  @BuiltValueField(wireName: r'success')
  bool get success;

  VerifyWebhookResponse._();

  factory VerifyWebhookResponse([void updates(VerifyWebhookResponseBuilder b)]) = _$VerifyWebhookResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(VerifyWebhookResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<VerifyWebhookResponse> get serializer => _$VerifyWebhookResponseSerializer();
}

class _$VerifyWebhookResponseSerializer implements PrimitiveSerializer<VerifyWebhookResponse> {
  @override
  final Iterable<Type> types = const [VerifyWebhookResponse, _$VerifyWebhookResponse];

  @override
  final String wireName = r'VerifyWebhookResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    VerifyWebhookResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'detail';
    yield serializers.serialize(
      object.detail,
      specifiedType: const FullType(String),
    );
    yield r'success';
    yield serializers.serialize(
      object.success,
      specifiedType: const FullType(bool),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    VerifyWebhookResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required VerifyWebhookResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'detail':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.detail = valueDes;
          break;
        case r'success':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.success = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  VerifyWebhookResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = VerifyWebhookResponseBuilder();
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

