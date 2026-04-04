//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/task_status.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'task_status_update_event.g.dart';

/// An event sent by the agent to notify the client of a change in a task's status.
///
/// Properties:
/// * [contextId] - The ID of the context that the task belongs to.
/// * [metadata] - Metadata associated with the task update.
/// * [status] - The new status of the task.
/// * [taskId] - The ID of the task that has changed.
@BuiltValue()
abstract class TaskStatusUpdateEvent implements Built<TaskStatusUpdateEvent, TaskStatusUpdateEventBuilder> {
  /// The ID of the context that the task belongs to.
  @BuiltValueField(wireName: r'contextId')
  String get contextId;

  /// Metadata associated with the task update.
  @BuiltValueField(wireName: r'metadata')
  BuiltMap<String, JsonObject?>? get metadata;

  /// The new status of the task.
  @BuiltValueField(wireName: r'status')
  TaskStatus get status;

  /// The ID of the task that has changed.
  @BuiltValueField(wireName: r'taskId')
  String get taskId;

  TaskStatusUpdateEvent._();

  factory TaskStatusUpdateEvent([void updates(TaskStatusUpdateEventBuilder b)]) = _$TaskStatusUpdateEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TaskStatusUpdateEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TaskStatusUpdateEvent> get serializer => _$TaskStatusUpdateEventSerializer();
}

class _$TaskStatusUpdateEventSerializer implements PrimitiveSerializer<TaskStatusUpdateEvent> {
  @override
  final Iterable<Type> types = const [TaskStatusUpdateEvent, _$TaskStatusUpdateEvent];

  @override
  final String wireName = r'TaskStatusUpdateEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TaskStatusUpdateEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'contextId';
    yield serializers.serialize(
      object.contextId,
      specifiedType: const FullType(String),
    );
    if (object.metadata != null) {
      yield r'metadata';
      yield serializers.serialize(
        object.metadata,
        specifiedType: const FullType.nullable(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
      );
    }
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(TaskStatus),
    );
    yield r'taskId';
    yield serializers.serialize(
      object.taskId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    TaskStatusUpdateEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TaskStatusUpdateEventBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'contextId':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.contextId = valueDes;
          break;
        case r'metadata':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
          ) as BuiltMap<String, JsonObject?>?;
          if (valueDes == null) continue;
          result.metadata.replace(valueDes);
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(TaskStatus),
          ) as TaskStatus;
          result.status.replace(valueDes);
          break;
        case r'taskId':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.taskId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TaskStatusUpdateEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TaskStatusUpdateEventBuilder();
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

