//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'get_task_push_notification_config_request.g.dart';

/// Request for the `GetTaskPushNotificationConfig` method.
///
/// Properties:
/// * [id] - The resource ID of the configuration to retrieve.
/// * [taskId] - The parent task resource ID.
/// * [tenant] - Optional tenant ID.
@BuiltValue()
abstract class GetTaskPushNotificationConfigRequest
    implements
        Built<GetTaskPushNotificationConfigRequest,
            GetTaskPushNotificationConfigRequestBuilder> {
  /// The resource ID of the configuration to retrieve.
  @BuiltValueField(wireName: r'id')
  String get id;

  /// The parent task resource ID.
  @BuiltValueField(wireName: r'taskId')
  String get taskId;

  /// Optional tenant ID.
  @BuiltValueField(wireName: r'tenant')
  String? get tenant;

  GetTaskPushNotificationConfigRequest._();

  factory GetTaskPushNotificationConfigRequest(
          [void updates(GetTaskPushNotificationConfigRequestBuilder b)]) =
      _$GetTaskPushNotificationConfigRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(GetTaskPushNotificationConfigRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<GetTaskPushNotificationConfigRequest> get serializer =>
      _$GetTaskPushNotificationConfigRequestSerializer();
}

class _$GetTaskPushNotificationConfigRequestSerializer
    implements PrimitiveSerializer<GetTaskPushNotificationConfigRequest> {
  @override
  final Iterable<Type> types = const [
    GetTaskPushNotificationConfigRequest,
    _$GetTaskPushNotificationConfigRequest
  ];

  @override
  final String wireName = r'GetTaskPushNotificationConfigRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    GetTaskPushNotificationConfigRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
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
    GetTaskPushNotificationConfigRequest object, {
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
    required GetTaskPushNotificationConfigRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
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
  GetTaskPushNotificationConfigRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = GetTaskPushNotificationConfigRequestBuilder();
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
