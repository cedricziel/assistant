//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'list_task_push_notification_configs_request.g.dart';

/// Request for the `ListTaskPushNotificationConfigs` method.
///
/// Properties:
/// * [pageSize] - Max number of configurations to return.
/// * [pageToken] - Page token from a previous call.
/// * [taskId] - The parent task resource ID.
/// * [tenant] - Optional tenant ID.
@BuiltValue()
abstract class ListTaskPushNotificationConfigsRequest
    implements
        Built<ListTaskPushNotificationConfigsRequest,
            ListTaskPushNotificationConfigsRequestBuilder> {
  /// Max number of configurations to return.
  @BuiltValueField(wireName: r'pageSize')
  int? get pageSize;

  /// Page token from a previous call.
  @BuiltValueField(wireName: r'pageToken')
  String? get pageToken;

  /// The parent task resource ID.
  @BuiltValueField(wireName: r'taskId')
  String get taskId;

  /// Optional tenant ID.
  @BuiltValueField(wireName: r'tenant')
  String? get tenant;

  ListTaskPushNotificationConfigsRequest._();

  factory ListTaskPushNotificationConfigsRequest(
          [void updates(ListTaskPushNotificationConfigsRequestBuilder b)]) =
      _$ListTaskPushNotificationConfigsRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ListTaskPushNotificationConfigsRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ListTaskPushNotificationConfigsRequest> get serializer =>
      _$ListTaskPushNotificationConfigsRequestSerializer();
}

class _$ListTaskPushNotificationConfigsRequestSerializer
    implements PrimitiveSerializer<ListTaskPushNotificationConfigsRequest> {
  @override
  final Iterable<Type> types = const [
    ListTaskPushNotificationConfigsRequest,
    _$ListTaskPushNotificationConfigsRequest
  ];

  @override
  final String wireName = r'ListTaskPushNotificationConfigsRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ListTaskPushNotificationConfigsRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.pageSize != null) {
      yield r'pageSize';
      yield serializers.serialize(
        object.pageSize,
        specifiedType: const FullType.nullable(int),
      );
    }
    if (object.pageToken != null) {
      yield r'pageToken';
      yield serializers.serialize(
        object.pageToken,
        specifiedType: const FullType.nullable(String),
      );
    }
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
    ListTaskPushNotificationConfigsRequest object, {
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
    required ListTaskPushNotificationConfigsRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'pageSize':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.pageSize = valueDes;
          break;
        case r'pageToken':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.pageToken = valueDes;
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
  ListTaskPushNotificationConfigsRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ListTaskPushNotificationConfigsRequestBuilder();
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
