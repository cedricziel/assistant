// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_task_push_notification_config_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateTaskPushNotificationConfigRequest
    extends CreateTaskPushNotificationConfigRequest {
  @override
  final PushNotificationConfig config;
  @override
  final String taskId;
  @override
  final String? tenant;

  factory _$CreateTaskPushNotificationConfigRequest(
          [void Function(CreateTaskPushNotificationConfigRequestBuilder)?
              updates]) =>
      (CreateTaskPushNotificationConfigRequestBuilder()..update(updates))
          ._build();

  _$CreateTaskPushNotificationConfigRequest._(
      {required this.config, required this.taskId, this.tenant})
      : super._();
  @override
  CreateTaskPushNotificationConfigRequest rebuild(
          void Function(CreateTaskPushNotificationConfigRequestBuilder)
              updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateTaskPushNotificationConfigRequestBuilder toBuilder() =>
      CreateTaskPushNotificationConfigRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateTaskPushNotificationConfigRequest &&
        config == other.config &&
        taskId == other.taskId &&
        tenant == other.tenant;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, config.hashCode);
    _$hash = $jc(_$hash, taskId.hashCode);
    _$hash = $jc(_$hash, tenant.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
            r'CreateTaskPushNotificationConfigRequest')
          ..add('config', config)
          ..add('taskId', taskId)
          ..add('tenant', tenant))
        .toString();
  }
}

class CreateTaskPushNotificationConfigRequestBuilder
    implements
        Builder<CreateTaskPushNotificationConfigRequest,
            CreateTaskPushNotificationConfigRequestBuilder> {
  _$CreateTaskPushNotificationConfigRequest? _$v;

  PushNotificationConfigBuilder? _config;
  PushNotificationConfigBuilder get config =>
      _$this._config ??= PushNotificationConfigBuilder();
  set config(PushNotificationConfigBuilder? config) => _$this._config = config;

  String? _taskId;
  String? get taskId => _$this._taskId;
  set taskId(String? taskId) => _$this._taskId = taskId;

  String? _tenant;
  String? get tenant => _$this._tenant;
  set tenant(String? tenant) => _$this._tenant = tenant;

  CreateTaskPushNotificationConfigRequestBuilder() {
    CreateTaskPushNotificationConfigRequest._defaults(this);
  }

  CreateTaskPushNotificationConfigRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _config = $v.config.toBuilder();
      _taskId = $v.taskId;
      _tenant = $v.tenant;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateTaskPushNotificationConfigRequest other) {
    _$v = other as _$CreateTaskPushNotificationConfigRequest;
  }

  @override
  void update(
      void Function(CreateTaskPushNotificationConfigRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateTaskPushNotificationConfigRequest build() => _build();

  _$CreateTaskPushNotificationConfigRequest _build() {
    _$CreateTaskPushNotificationConfigRequest _$result;
    try {
      _$result = _$v ??
          _$CreateTaskPushNotificationConfigRequest._(
            config: config.build(),
            taskId: BuiltValueNullFieldError.checkNotNull(
                taskId, r'CreateTaskPushNotificationConfigRequest', 'taskId'),
            tenant: tenant,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'config';
        config.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'CreateTaskPushNotificationConfigRequest',
            _$failedField,
            e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
