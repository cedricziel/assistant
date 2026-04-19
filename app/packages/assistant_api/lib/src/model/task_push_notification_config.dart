//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/push_notification_config.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'task_push_notification_config.g.dart';

/// A container associating a push notification configuration with a specific task.
///
/// Properties:
/// * [pushNotificationConfig] - The push notification configuration details.
/// * [taskId] - The ID of the task this configuration is associated with.
/// * [tenant] - Tenant ID.
@BuiltValue()
abstract class TaskPushNotificationConfig implements Built<TaskPushNotificationConfig, TaskPushNotificationConfigBuilder> {
  /// The push notification configuration details.
  @BuiltValueField(wireName: r'pushNotificationConfig')
  PushNotificationConfig get pushNotificationConfig;

  /// The ID of the task this configuration is associated with.
  @BuiltValueField(wireName: r'taskId')
  String get taskId;

  /// Tenant ID.
  @BuiltValueField(wireName: r'tenant')
  String? get tenant;

  TaskPushNotificationConfig._();

  factory TaskPushNotificationConfig([void updates(TaskPushNotificationConfigBuilder b)]) = _$TaskPushNotificationConfig;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TaskPushNotificationConfigBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TaskPushNotificationConfig> get serializer => _$TaskPushNotificationConfigSerializer();
}

class _$TaskPushNotificationConfigSerializer implements PrimitiveSerializer<TaskPushNotificationConfig> {
  @override
  final Iterable<Type> types = const [TaskPushNotificationConfig, _$TaskPushNotificationConfig];

  @override
  final String wireName = r'TaskPushNotificationConfig';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TaskPushNotificationConfig object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'pushNotificationConfig';
    yield serializers.serialize(
      object.pushNotificationConfig,
      specifiedType: const FullType(PushNotificationConfig),
    );
    yield r'taskId';
    yield serializers.serialize(
      object.taskId,
      specifiedType: const FullType(String),
    );
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
    TaskPushNotificationConfig object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TaskPushNotificationConfigBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'pushNotificationConfig':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(PushNotificationConfig),
          ) as PushNotificationConfig;
          result.pushNotificationConfig.replace(valueDes);
          break;
        case r'taskId':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.taskId = valueDes;
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
  TaskPushNotificationConfig deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TaskPushNotificationConfigBuilder();
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

