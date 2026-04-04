// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'get_task_push_notification_config_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$GetTaskPushNotificationConfigRequest
    extends GetTaskPushNotificationConfigRequest {
  @override
  final String id;
  @override
  final String taskId;
  @override
  final String? tenant;

  factory _$GetTaskPushNotificationConfigRequest(
          [void Function(GetTaskPushNotificationConfigRequestBuilder)?
              updates]) =>
      (GetTaskPushNotificationConfigRequestBuilder()..update(updates))._build();

  _$GetTaskPushNotificationConfigRequest._(
      {required this.id, required this.taskId, this.tenant})
      : super._();
  @override
  GetTaskPushNotificationConfigRequest rebuild(
          void Function(GetTaskPushNotificationConfigRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  GetTaskPushNotificationConfigRequestBuilder toBuilder() =>
      GetTaskPushNotificationConfigRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is GetTaskPushNotificationConfigRequest &&
        id == other.id &&
        taskId == other.taskId &&
        tenant == other.tenant;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, taskId.hashCode);
    _$hash = $jc(_$hash, tenant.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'GetTaskPushNotificationConfigRequest')
          ..add('id', id)
          ..add('taskId', taskId)
          ..add('tenant', tenant))
        .toString();
  }
}

class GetTaskPushNotificationConfigRequestBuilder
    implements
        Builder<GetTaskPushNotificationConfigRequest,
            GetTaskPushNotificationConfigRequestBuilder> {
  _$GetTaskPushNotificationConfigRequest? _$v;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _taskId;
  String? get taskId => _$this._taskId;
  set taskId(String? taskId) => _$this._taskId = taskId;

  String? _tenant;
  String? get tenant => _$this._tenant;
  set tenant(String? tenant) => _$this._tenant = tenant;

  GetTaskPushNotificationConfigRequestBuilder() {
    GetTaskPushNotificationConfigRequest._defaults(this);
  }

  GetTaskPushNotificationConfigRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _id = $v.id;
      _taskId = $v.taskId;
      _tenant = $v.tenant;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(GetTaskPushNotificationConfigRequest other) {
    _$v = other as _$GetTaskPushNotificationConfigRequest;
  }

  @override
  void update(
      void Function(GetTaskPushNotificationConfigRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  GetTaskPushNotificationConfigRequest build() => _build();

  _$GetTaskPushNotificationConfigRequest _build() {
    final _$result = _$v ??
        _$GetTaskPushNotificationConfigRequest._(
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'GetTaskPushNotificationConfigRequest', 'id'),
          taskId: BuiltValueNullFieldError.checkNotNull(
              taskId, r'GetTaskPushNotificationConfigRequest', 'taskId'),
          tenant: tenant,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
