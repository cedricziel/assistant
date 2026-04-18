//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'task_state.g.dart';

class TaskState extends EnumClass {
  /// Defines the possible lifecycle states of a `Task`.
  @BuiltValueEnumConst(wireName: r'TASK_STATE_UNSPECIFIED')
  static const TaskState TASK_STATE_UNSPECIFIED = _$TASK_STATE_UNSPECIFIED;

  /// Defines the possible lifecycle states of a `Task`.
  @BuiltValueEnumConst(wireName: r'TASK_STATE_SUBMITTED')
  static const TaskState TASK_STATE_SUBMITTED = _$TASK_STATE_SUBMITTED;

  /// Defines the possible lifecycle states of a `Task`.
  @BuiltValueEnumConst(wireName: r'TASK_STATE_WORKING')
  static const TaskState TASK_STATE_WORKING = _$TASK_STATE_WORKING;

  /// Defines the possible lifecycle states of a `Task`.
  @BuiltValueEnumConst(wireName: r'TASK_STATE_COMPLETED')
  static const TaskState TASK_STATE_COMPLETED = _$TASK_STATE_COMPLETED;

  /// Defines the possible lifecycle states of a `Task`.
  @BuiltValueEnumConst(wireName: r'TASK_STATE_FAILED')
  static const TaskState TASK_STATE_FAILED = _$TASK_STATE_FAILED;

  /// Defines the possible lifecycle states of a `Task`.
  @BuiltValueEnumConst(wireName: r'TASK_STATE_CANCELED')
  static const TaskState TASK_STATE_CANCELED = _$TASK_STATE_CANCELED;

  /// Defines the possible lifecycle states of a `Task`.
  @BuiltValueEnumConst(wireName: r'TASK_STATE_INPUT_REQUIRED')
  static const TaskState TASK_STATE_INPUT_REQUIRED =
      _$TASK_STATE_INPUT_REQUIRED;

  /// Defines the possible lifecycle states of a `Task`.
  @BuiltValueEnumConst(wireName: r'TASK_STATE_REJECTED')
  static const TaskState TASK_STATE_REJECTED = _$TASK_STATE_REJECTED;

  /// Defines the possible lifecycle states of a `Task`.
  @BuiltValueEnumConst(wireName: r'TASK_STATE_AUTH_REQUIRED')
  static const TaskState TASK_STATE_AUTH_REQUIRED = _$TASK_STATE_AUTH_REQUIRED;

  /// Defines the possible lifecycle states of a `Task`.
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const TaskState unknownDefaultOpenApi = _$unknownDefaultOpenApi;

  static Serializer<TaskState> get serializer => _$taskStateSerializer;

  const TaskState._(String name) : super(name);

  static BuiltSet<TaskState> get values => _$values;
  static TaskState valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class TaskStateMixin = Object with _$TaskStateMixin;
