//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/push_notification_config.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'send_message_configuration.g.dart';

/// Configuration of a send message request.
///
/// Properties:
/// * [acceptedOutputModes] - Media types the client accepts for response parts.
/// * [blocking] - If true, wait until the task reaches a terminal or interrupted state.
/// * [historyLength] - Max number of recent messages to include in the response.
/// * [pushNotificationConfig] - Push notification configuration for task updates.
@BuiltValue()
abstract class SendMessageConfiguration implements Built<SendMessageConfiguration, SendMessageConfigurationBuilder> {
  /// Media types the client accepts for response parts.
  @BuiltValueField(wireName: r'acceptedOutputModes')
  BuiltList<String>? get acceptedOutputModes;

  /// If true, wait until the task reaches a terminal or interrupted state.
  @BuiltValueField(wireName: r'blocking')
  bool? get blocking;

  /// Max number of recent messages to include in the response.
  @BuiltValueField(wireName: r'historyLength')
  int? get historyLength;

  /// Push notification configuration for task updates.
  @BuiltValueField(wireName: r'pushNotificationConfig')
  PushNotificationConfig? get pushNotificationConfig;

  SendMessageConfiguration._();

  factory SendMessageConfiguration([void updates(SendMessageConfigurationBuilder b)]) = _$SendMessageConfiguration;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SendMessageConfigurationBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SendMessageConfiguration> get serializer => _$SendMessageConfigurationSerializer();
}

class _$SendMessageConfigurationSerializer implements PrimitiveSerializer<SendMessageConfiguration> {
  @override
  final Iterable<Type> types = const [SendMessageConfiguration, _$SendMessageConfiguration];

  @override
  final String wireName = r'SendMessageConfiguration';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SendMessageConfiguration object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.acceptedOutputModes != null) {
      yield r'acceptedOutputModes';
      yield serializers.serialize(
        object.acceptedOutputModes,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    if (object.blocking != null) {
      yield r'blocking';
      yield serializers.serialize(
        object.blocking,
        specifiedType: const FullType(bool),
      );
    }
    if (object.historyLength != null) {
      yield r'historyLength';
      yield serializers.serialize(
        object.historyLength,
        specifiedType: const FullType.nullable(int),
      );
    }
    if (object.pushNotificationConfig != null) {
      yield r'pushNotificationConfig';
      yield serializers.serialize(
        object.pushNotificationConfig,
        specifiedType: const FullType.nullable(PushNotificationConfig),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    SendMessageConfiguration object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SendMessageConfigurationBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'acceptedOutputModes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.acceptedOutputModes.replace(valueDes);
          break;
        case r'blocking':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.blocking = valueDes;
          break;
        case r'historyLength':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.historyLength = valueDes;
          break;
        case r'pushNotificationConfig':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(PushNotificationConfig),
          ) as PushNotificationConfig?;
          if (valueDes == null) continue;
          result.pushNotificationConfig.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SendMessageConfiguration deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SendMessageConfigurationBuilder();
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

