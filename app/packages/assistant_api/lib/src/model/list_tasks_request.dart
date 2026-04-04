//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/task_state.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'list_tasks_request.g.dart';

/// Request for the `ListTasks` method.
///
/// Properties:
/// * [contextId] - Filter tasks by context ID.
/// * [historyLength] - Max number of messages to include in each task's history.
/// * [includeArtifacts] - Whether to include artifacts in returned tasks.
/// * [pageSize] - Max number of tasks to return (1..=100, default 50).
/// * [pageToken] - Page token from a previous `ListTasks` call.
/// * [status] - Filter tasks by current status state.
/// * [statusTimestampAfter] - Filter tasks with status updated after this timestamp.
/// * [tenant] - Tenant ID.
@BuiltValue()
abstract class ListTasksRequest implements Built<ListTasksRequest, ListTasksRequestBuilder> {
  /// Filter tasks by context ID.
  @BuiltValueField(wireName: r'contextId')
  String? get contextId;

  /// Max number of messages to include in each task's history.
  @BuiltValueField(wireName: r'historyLength')
  int? get historyLength;

  /// Whether to include artifacts in returned tasks.
  @BuiltValueField(wireName: r'includeArtifacts')
  bool? get includeArtifacts;

  /// Max number of tasks to return (1..=100, default 50).
  @BuiltValueField(wireName: r'pageSize')
  int? get pageSize;

  /// Page token from a previous `ListTasks` call.
  @BuiltValueField(wireName: r'pageToken')
  String? get pageToken;

  /// Filter tasks by current status state.
  @BuiltValueField(wireName: r'status')
  TaskState? get status;
  // enum statusEnum {  TASK_STATE_UNSPECIFIED,  TASK_STATE_SUBMITTED,  TASK_STATE_WORKING,  TASK_STATE_COMPLETED,  TASK_STATE_FAILED,  TASK_STATE_CANCELED,  TASK_STATE_INPUT_REQUIRED,  TASK_STATE_REJECTED,  TASK_STATE_AUTH_REQUIRED,  };

  /// Filter tasks with status updated after this timestamp.
  @BuiltValueField(wireName: r'statusTimestampAfter')
  DateTime? get statusTimestampAfter;

  /// Tenant ID.
  @BuiltValueField(wireName: r'tenant')
  String? get tenant;

  ListTasksRequest._();

  factory ListTasksRequest([void updates(ListTasksRequestBuilder b)]) = _$ListTasksRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ListTasksRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ListTasksRequest> get serializer => _$ListTasksRequestSerializer();
}

class _$ListTasksRequestSerializer implements PrimitiveSerializer<ListTasksRequest> {
  @override
  final Iterable<Type> types = const [ListTasksRequest, _$ListTasksRequest];

  @override
  final String wireName = r'ListTasksRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ListTasksRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.contextId != null) {
      yield r'contextId';
      yield serializers.serialize(
        object.contextId,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.historyLength != null) {
      yield r'historyLength';
      yield serializers.serialize(
        object.historyLength,
        specifiedType: const FullType.nullable(int),
      );
    }
    if (object.includeArtifacts != null) {
      yield r'includeArtifacts';
      yield serializers.serialize(
        object.includeArtifacts,
        specifiedType: const FullType.nullable(bool),
      );
    }
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
    if (object.status != null) {
      yield r'status';
      yield serializers.serialize(
        object.status,
        specifiedType: const FullType.nullable(TaskState),
      );
    }
    if (object.statusTimestampAfter != null) {
      yield r'statusTimestampAfter';
      yield serializers.serialize(
        object.statusTimestampAfter,
        specifiedType: const FullType.nullable(DateTime),
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
    ListTasksRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ListTasksRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'contextId':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.contextId = valueDes;
          break;
        case r'historyLength':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.historyLength = valueDes;
          break;
        case r'includeArtifacts':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(bool),
          ) as bool?;
          if (valueDes == null) continue;
          result.includeArtifacts = valueDes;
          break;
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
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(TaskState),
          ) as TaskState?;
          if (valueDes == null) continue;
          result.status = valueDes;
          break;
        case r'statusTimestampAfter':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(DateTime),
          ) as DateTime?;
          if (valueDes == null) continue;
          result.statusTimestampAfter = valueDes;
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
  ListTasksRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ListTasksRequestBuilder();
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

