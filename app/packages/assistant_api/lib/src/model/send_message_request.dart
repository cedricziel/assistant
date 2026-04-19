//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/send_message_configuration.dart';
import 'package:assistant_api/src/model/message.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'send_message_request.g.dart';

/// Request for the `SendMessage` and `SendStreamingMessage` methods.
///
/// Properties:
/// * [configuration] - Configuration for the send request.
/// * [message] - The message to send to the agent.
/// * [metadata] - Additional context or parameters.
/// * [tenant] - Optional tenant ID.
@BuiltValue()
abstract class SendMessageRequest implements Built<SendMessageRequest, SendMessageRequestBuilder> {
  /// Configuration for the send request.
  @BuiltValueField(wireName: r'configuration')
  SendMessageConfiguration? get configuration;

  /// The message to send to the agent.
  @BuiltValueField(wireName: r'message')
  Message get message;

  /// Additional context or parameters.
  @BuiltValueField(wireName: r'metadata')
  BuiltMap<String, JsonObject?>? get metadata;

  /// Optional tenant ID.
  @BuiltValueField(wireName: r'tenant')
  String? get tenant;

  SendMessageRequest._();

  factory SendMessageRequest([void updates(SendMessageRequestBuilder b)]) = _$SendMessageRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SendMessageRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SendMessageRequest> get serializer => _$SendMessageRequestSerializer();
}

class _$SendMessageRequestSerializer implements PrimitiveSerializer<SendMessageRequest> {
  @override
  final Iterable<Type> types = const [SendMessageRequest, _$SendMessageRequest];

  @override
  final String wireName = r'SendMessageRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SendMessageRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.configuration != null) {
      yield r'configuration';
      yield serializers.serialize(
        object.configuration,
        specifiedType: const FullType.nullable(SendMessageConfiguration),
      );
    }
    yield r'message';
    yield serializers.serialize(
      object.message,
      specifiedType: const FullType(Message),
    );
    if (object.metadata != null) {
      yield r'metadata';
      yield serializers.serialize(
        object.metadata,
        specifiedType: const FullType.nullable(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
      );
    }
    if (object.tenant != null) {
      yield r'tenant';
      yield serializers.serialize(
        object.tenant,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    SendMessageRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SendMessageRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'configuration':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(SendMessageConfiguration),
          ) as SendMessageConfiguration?;
          if (valueDes == null) continue;
          result.configuration.replace(valueDes);
          break;
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(Message),
          ) as Message;
          result.message.replace(valueDes);
          break;
        case r'metadata':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
          ) as BuiltMap<String, JsonObject?>?;
          if (valueDes == null) continue;
          result.metadata.replace(valueDes);
          break;
        case r'tenant':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.tenant = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SendMessageRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SendMessageRequestBuilder();
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

