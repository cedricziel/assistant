//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/task_state.dart';
import 'package:assistant_api/src/model/message.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'task_status.g.dart';

/// A container for the status of a task.
///
/// Properties:
/// * [message] - A message associated with the status.
/// * [state] - The current state of this task.
/// * [timestamp] - ISO 8601 timestamp when the status was recorded.
@BuiltValue()
abstract class TaskStatus implements Built<TaskStatus, TaskStatusBuilder> {
  /// A message associated with the status.
  @BuiltValueField(wireName: r'message')
  Message? get message;

  /// The current state of this task.
  @BuiltValueField(wireName: r'state')
  TaskState get state;
  // enum stateEnum {  TASK_STATE_UNSPECIFIED,  TASK_STATE_SUBMITTED,  TASK_STATE_WORKING,  TASK_STATE_COMPLETED,  TASK_STATE_FAILED,  TASK_STATE_CANCELED,  TASK_STATE_INPUT_REQUIRED,  TASK_STATE_REJECTED,  TASK_STATE_AUTH_REQUIRED,  };

  /// ISO 8601 timestamp when the status was recorded.
  @BuiltValueField(wireName: r'timestamp')
  DateTime? get timestamp;

  TaskStatus._();

  factory TaskStatus([void updates(TaskStatusBuilder b)]) = _$TaskStatus;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TaskStatusBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TaskStatus> get serializer => _$TaskStatusSerializer();
}

class _$TaskStatusSerializer implements PrimitiveSerializer<TaskStatus> {
  @override
  final Iterable<Type> types = const [TaskStatus, _$TaskStatus];

  @override
  final String wireName = r'TaskStatus';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TaskStatus object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.message != null) {
      yield r'message';
      yield serializers.serialize(
        object.message,
        specifiedType: const FullType.nullable(Message),
      );
    }
    yield r'state';
    yield serializers.serialize(
      object.state,
      specifiedType: const FullType(TaskState),
    );
    if (object.timestamp != null) {
      yield r'timestamp';
      yield serializers.serialize(
        object.timestamp,
        specifiedType: const FullType.nullable(DateTime),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    TaskStatus object, {
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
    required TaskStatusBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(Message),
          ) as Message?;
          if (valueDes == null) continue;
          result.message.replace(valueDes);
          break;
        case r'state':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(TaskState),
          ) as TaskState;
          result.state = valueDes;
          break;
        case r'timestamp':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(DateTime),
          ) as DateTime?;
          if (valueDes == null) continue;
          result.timestamp = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TaskStatus deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TaskStatusBuilder();
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
