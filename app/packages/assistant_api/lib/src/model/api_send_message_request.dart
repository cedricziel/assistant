//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'api_send_message_request.g.dart';

/// Body for `POST /api/conversations/{id}/messages`.
///
/// Properties:
/// * [message] - The message text to send to the assistant.
@BuiltValue()
abstract class ApiSendMessageRequest implements Built<ApiSendMessageRequest, ApiSendMessageRequestBuilder> {
  /// The message text to send to the assistant.
  @BuiltValueField(wireName: r'message')
  String get message;

  ApiSendMessageRequest._();

  factory ApiSendMessageRequest([void updates(ApiSendMessageRequestBuilder b)]) = _$ApiSendMessageRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApiSendMessageRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApiSendMessageRequest> get serializer => _$ApiSendMessageRequestSerializer();
}

class _$ApiSendMessageRequestSerializer implements PrimitiveSerializer<ApiSendMessageRequest> {
  @override
  final Iterable<Type> types = const [ApiSendMessageRequest, _$ApiSendMessageRequest];

  @override
  final String wireName = r'ApiSendMessageRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApiSendMessageRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'message';
    yield serializers.serialize(
      object.message,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApiSendMessageRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApiSendMessageRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.message = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApiSendMessageRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApiSendMessageRequestBuilder();
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

