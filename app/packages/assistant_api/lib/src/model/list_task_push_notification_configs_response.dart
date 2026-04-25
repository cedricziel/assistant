//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/task_push_notification_config.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'list_task_push_notification_configs_response.g.dart';

/// Response for the `ListTaskPushNotificationConfigs` method.
///
/// Properties:
/// * [configs] - The list of push notification configurations.
/// * [nextPageToken] - A token to retrieve the next page of results.
@BuiltValue()
abstract class ListTaskPushNotificationConfigsResponse
    implements
        Built<ListTaskPushNotificationConfigsResponse,
            ListTaskPushNotificationConfigsResponseBuilder> {
  /// The list of push notification configurations.
  @BuiltValueField(wireName: r'configs')
  BuiltList<TaskPushNotificationConfig> get configs;

  /// A token to retrieve the next page of results.
  @BuiltValueField(wireName: r'nextPageToken')
  String? get nextPageToken;

  ListTaskPushNotificationConfigsResponse._();

  factory ListTaskPushNotificationConfigsResponse(
          [void updates(ListTaskPushNotificationConfigsResponseBuilder b)]) =
      _$ListTaskPushNotificationConfigsResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ListTaskPushNotificationConfigsResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ListTaskPushNotificationConfigsResponse> get serializer =>
      _$ListTaskPushNotificationConfigsResponseSerializer();
}

class _$ListTaskPushNotificationConfigsResponseSerializer
    implements PrimitiveSerializer<ListTaskPushNotificationConfigsResponse> {
  @override
  final Iterable<Type> types = const [
    ListTaskPushNotificationConfigsResponse,
    _$ListTaskPushNotificationConfigsResponse
  ];

  @override
  final String wireName = r'ListTaskPushNotificationConfigsResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ListTaskPushNotificationConfigsResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'configs';
    yield serializers.serialize(
      object.configs,
      specifiedType:
          const FullType(BuiltList, [FullType(TaskPushNotificationConfig)]),
    );
    if (object.nextPageToken != null) {
      yield r'nextPageToken';
      yield serializers.serialize(
        object.nextPageToken,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    ListTaskPushNotificationConfigsResponse object, {
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
    required ListTaskPushNotificationConfigsResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'configs':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(
                BuiltList, [FullType(TaskPushNotificationConfig)]),
          ) as BuiltList<TaskPushNotificationConfig>;
          result.configs.replace(valueDes);
          break;
        case r'nextPageToken':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.nextPageToken = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ListTaskPushNotificationConfigsResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ListTaskPushNotificationConfigsResponseBuilder();
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
