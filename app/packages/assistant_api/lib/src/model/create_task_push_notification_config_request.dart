//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/push_notification_config.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_task_push_notification_config_request.g.dart';

/// Request for the `CreateTaskPushNotificationConfig` method.
///
/// Properties:
/// * [config] - The configuration to create.
/// * [taskId] - The parent task resource ID.
/// * [tenant] - Optional tenant ID.
@BuiltValue()
abstract class CreateTaskPushNotificationConfigRequest implements Built<CreateTaskPushNotificationConfigRequest, CreateTaskPushNotificationConfigRequestBuilder> {
  /// The configuration to create.
  @BuiltValueField(wireName: r'config')
  PushNotificationConfig get config;

  /// The parent task resource ID.
  @BuiltValueField(wireName: r'taskId')
  String get taskId;

  /// Optional tenant ID.
  @BuiltValueField(wireName: r'tenant')
  String? get tenant;

  CreateTaskPushNotificationConfigRequest._();

  factory CreateTaskPushNotificationConfigRequest([void updates(CreateTaskPushNotificationConfigRequestBuilder b)]) = _$CreateTaskPushNotificationConfigRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateTaskPushNotificationConfigRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateTaskPushNotificationConfigRequest> get serializer => _$CreateTaskPushNotificationConfigRequestSerializer();
}

class _$CreateTaskPushNotificationConfigRequestSerializer implements PrimitiveSerializer<CreateTaskPushNotificationConfigRequest> {
  @override
  final Iterable<Type> types = const [CreateTaskPushNotificationConfigRequest, _$CreateTaskPushNotificationConfigRequest];

  @override
  final String wireName = r'CreateTaskPushNotificationConfigRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateTaskPushNotificationConfigRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'config';
    yield serializers.serialize(
      object.config,
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
    CreateTaskPushNotificationConfigRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required CreateTaskPushNotificationConfigRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'config':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(PushNotificationConfig),
          ) as PushNotificationConfig;
          result.config.replace(valueDes);
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
  CreateTaskPushNotificationConfigRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateTaskPushNotificationConfigRequestBuilder();
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

