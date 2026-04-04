// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'task_state.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const TaskState _$TASK_STATE_UNSPECIFIED =
    const TaskState._('TASK_STATE_UNSPECIFIED');
const TaskState _$TASK_STATE_SUBMITTED =
    const TaskState._('TASK_STATE_SUBMITTED');
const TaskState _$TASK_STATE_WORKING = const TaskState._('TASK_STATE_WORKING');
const TaskState _$TASK_STATE_COMPLETED =
    const TaskState._('TASK_STATE_COMPLETED');
const TaskState _$TASK_STATE_FAILED = const TaskState._('TASK_STATE_FAILED');
const TaskState _$TASK_STATE_CANCELED =
    const TaskState._('TASK_STATE_CANCELED');
const TaskState _$TASK_STATE_INPUT_REQUIRED =
    const TaskState._('TASK_STATE_INPUT_REQUIRED');
const TaskState _$TASK_STATE_REJECTED =
    const TaskState._('TASK_STATE_REJECTED');
const TaskState _$TASK_STATE_AUTH_REQUIRED =
    const TaskState._('TASK_STATE_AUTH_REQUIRED');
const TaskState _$unknownDefaultOpenApi =
    const TaskState._('unknownDefaultOpenApi');

TaskState _$valueOf(String name) {
  switch (name) {
    case 'TASK_STATE_UNSPECIFIED':
      return _$TASK_STATE_UNSPECIFIED;
    case 'TASK_STATE_SUBMITTED':
      return _$TASK_STATE_SUBMITTED;
    case 'TASK_STATE_WORKING':
      return _$TASK_STATE_WORKING;
    case 'TASK_STATE_COMPLETED':
      return _$TASK_STATE_COMPLETED;
    case 'TASK_STATE_FAILED':
      return _$TASK_STATE_FAILED;
    case 'TASK_STATE_CANCELED':
      return _$TASK_STATE_CANCELED;
    case 'TASK_STATE_INPUT_REQUIRED':
      return _$TASK_STATE_INPUT_REQUIRED;
    case 'TASK_STATE_REJECTED':
      return _$TASK_STATE_REJECTED;
    case 'TASK_STATE_AUTH_REQUIRED':
      return _$TASK_STATE_AUTH_REQUIRED;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<TaskState> _$values = BuiltSet<TaskState>(const <TaskState>[
  _$TASK_STATE_UNSPECIFIED,
  _$TASK_STATE_SUBMITTED,
  _$TASK_STATE_WORKING,
  _$TASK_STATE_COMPLETED,
  _$TASK_STATE_FAILED,
  _$TASK_STATE_CANCELED,
  _$TASK_STATE_INPUT_REQUIRED,
  _$TASK_STATE_REJECTED,
  _$TASK_STATE_AUTH_REQUIRED,
  _$unknownDefaultOpenApi,
]);

class _$TaskStateMeta {
  const _$TaskStateMeta();
  TaskState get TASK_STATE_UNSPECIFIED => _$TASK_STATE_UNSPECIFIED;
  TaskState get TASK_STATE_SUBMITTED => _$TASK_STATE_SUBMITTED;
  TaskState get TASK_STATE_WORKING => _$TASK_STATE_WORKING;
  TaskState get TASK_STATE_COMPLETED => _$TASK_STATE_COMPLETED;
  TaskState get TASK_STATE_FAILED => _$TASK_STATE_FAILED;
  TaskState get TASK_STATE_CANCELED => _$TASK_STATE_CANCELED;
  TaskState get TASK_STATE_INPUT_REQUIRED => _$TASK_STATE_INPUT_REQUIRED;
  TaskState get TASK_STATE_REJECTED => _$TASK_STATE_REJECTED;
  TaskState get TASK_STATE_AUTH_REQUIRED => _$TASK_STATE_AUTH_REQUIRED;
  TaskState get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  TaskState valueOf(String name) => _$valueOf(name);
  BuiltSet<TaskState> get values => _$values;
}

abstract class _$TaskStateMixin {
  // ignore: non_constant_identifier_names
  _$TaskStateMeta get TaskState => const _$TaskStateMeta();
}

Serializer<TaskState> _$taskStateSerializer = _$TaskStateSerializer();

class _$TaskStateSerializer implements PrimitiveSerializer<TaskState> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'TASK_STATE_UNSPECIFIED': 'TASK_STATE_UNSPECIFIED',
    'TASK_STATE_SUBMITTED': 'TASK_STATE_SUBMITTED',
    'TASK_STATE_WORKING': 'TASK_STATE_WORKING',
    'TASK_STATE_COMPLETED': 'TASK_STATE_COMPLETED',
    'TASK_STATE_FAILED': 'TASK_STATE_FAILED',
    'TASK_STATE_CANCELED': 'TASK_STATE_CANCELED',
    'TASK_STATE_INPUT_REQUIRED': 'TASK_STATE_INPUT_REQUIRED',
    'TASK_STATE_REJECTED': 'TASK_STATE_REJECTED',
    'TASK_STATE_AUTH_REQUIRED': 'TASK_STATE_AUTH_REQUIRED',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'TASK_STATE_UNSPECIFIED': 'TASK_STATE_UNSPECIFIED',
    'TASK_STATE_SUBMITTED': 'TASK_STATE_SUBMITTED',
    'TASK_STATE_WORKING': 'TASK_STATE_WORKING',
    'TASK_STATE_COMPLETED': 'TASK_STATE_COMPLETED',
    'TASK_STATE_FAILED': 'TASK_STATE_FAILED',
    'TASK_STATE_CANCELED': 'TASK_STATE_CANCELED',
    'TASK_STATE_INPUT_REQUIRED': 'TASK_STATE_INPUT_REQUIRED',
    'TASK_STATE_REJECTED': 'TASK_STATE_REJECTED',
    'TASK_STATE_AUTH_REQUIRED': 'TASK_STATE_AUTH_REQUIRED',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[TaskState];
  @override
  final String wireName = 'TaskState';

  @override
  Object serialize(Serializers serializers, TaskState object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  TaskState deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      TaskState.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
