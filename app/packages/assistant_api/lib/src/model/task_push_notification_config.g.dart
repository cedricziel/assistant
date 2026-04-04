// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'task_push_notification_config.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TaskPushNotificationConfig extends TaskPushNotificationConfig {
  @override
  final PushNotificationConfig pushNotificationConfig;
  @override
  final String taskId;
  @override
  final String? tenant;

  factory _$TaskPushNotificationConfig(
          [void Function(TaskPushNotificationConfigBuilder)? updates]) =>
      (TaskPushNotificationConfigBuilder()..update(updates))._build();

  _$TaskPushNotificationConfig._(
      {required this.pushNotificationConfig, required this.taskId, this.tenant})
      : super._();
  @override
  TaskPushNotificationConfig rebuild(
          void Function(TaskPushNotificationConfigBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TaskPushNotificationConfigBuilder toBuilder() =>
      TaskPushNotificationConfigBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TaskPushNotificationConfig &&
        pushNotificationConfig == other.pushNotificationConfig &&
        taskId == other.taskId &&
        tenant == other.tenant;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, pushNotificationConfig.hashCode);
    _$hash = $jc(_$hash, taskId.hashCode);
    _$hash = $jc(_$hash, tenant.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TaskPushNotificationConfig')
          ..add('pushNotificationConfig', pushNotificationConfig)
          ..add('taskId', taskId)
          ..add('tenant', tenant))
        .toString();
  }
}

class TaskPushNotificationConfigBuilder
    implements
        Builder<TaskPushNotificationConfig, TaskPushNotificationConfigBuilder> {
  _$TaskPushNotificationConfig? _$v;

  PushNotificationConfigBuilder? _pushNotificationConfig;
  PushNotificationConfigBuilder get pushNotificationConfig =>
      _$this._pushNotificationConfig ??= PushNotificationConfigBuilder();
  set pushNotificationConfig(
          PushNotificationConfigBuilder? pushNotificationConfig) =>
      _$this._pushNotificationConfig = pushNotificationConfig;

  String? _taskId;
  String? get taskId => _$this._taskId;
  set taskId(String? taskId) => _$this._taskId = taskId;

  String? _tenant;
  String? get tenant => _$this._tenant;
  set tenant(String? tenant) => _$this._tenant = tenant;

  TaskPushNotificationConfigBuilder() {
    TaskPushNotificationConfig._defaults(this);
  }

  TaskPushNotificationConfigBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _pushNotificationConfig = $v.pushNotificationConfig.toBuilder();
      _taskId = $v.taskId;
      _tenant = $v.tenant;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TaskPushNotificationConfig other) {
    _$v = other as _$TaskPushNotificationConfig;
  }

  @override
  void update(void Function(TaskPushNotificationConfigBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TaskPushNotificationConfig build() => _build();

  _$TaskPushNotificationConfig _build() {
    _$TaskPushNotificationConfig _$result;
    try {
      _$result = _$v ??
          _$TaskPushNotificationConfig._(
            pushNotificationConfig: pushNotificationConfig.build(),
            taskId: BuiltValueNullFieldError.checkNotNull(
                taskId, r'TaskPushNotificationConfig', 'taskId'),
            tenant: tenant,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'pushNotificationConfig';
        pushNotificationConfig.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'TaskPushNotificationConfig', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
