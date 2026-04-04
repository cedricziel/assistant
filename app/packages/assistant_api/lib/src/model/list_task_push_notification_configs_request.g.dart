// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'list_task_push_notification_configs_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ListTaskPushNotificationConfigsRequest
    extends ListTaskPushNotificationConfigsRequest {
  @override
  final int? pageSize;
  @override
  final String? pageToken;
  @override
  final String taskId;
  @override
  final String? tenant;

  factory _$ListTaskPushNotificationConfigsRequest(
          [void Function(ListTaskPushNotificationConfigsRequestBuilder)?
              updates]) =>
      (ListTaskPushNotificationConfigsRequestBuilder()..update(updates))
          ._build();

  _$ListTaskPushNotificationConfigsRequest._(
      {this.pageSize, this.pageToken, required this.taskId, this.tenant})
      : super._();
  @override
  ListTaskPushNotificationConfigsRequest rebuild(
          void Function(ListTaskPushNotificationConfigsRequestBuilder)
              updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ListTaskPushNotificationConfigsRequestBuilder toBuilder() =>
      ListTaskPushNotificationConfigsRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ListTaskPushNotificationConfigsRequest &&
        pageSize == other.pageSize &&
        pageToken == other.pageToken &&
        taskId == other.taskId &&
        tenant == other.tenant;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, pageSize.hashCode);
    _$hash = $jc(_$hash, pageToken.hashCode);
    _$hash = $jc(_$hash, taskId.hashCode);
    _$hash = $jc(_$hash, tenant.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
            r'ListTaskPushNotificationConfigsRequest')
          ..add('pageSize', pageSize)
          ..add('pageToken', pageToken)
          ..add('taskId', taskId)
          ..add('tenant', tenant))
        .toString();
  }
}

class ListTaskPushNotificationConfigsRequestBuilder
    implements
        Builder<ListTaskPushNotificationConfigsRequest,
            ListTaskPushNotificationConfigsRequestBuilder> {
  _$ListTaskPushNotificationConfigsRequest? _$v;

  int? _pageSize;
  int? get pageSize => _$this._pageSize;
  set pageSize(int? pageSize) => _$this._pageSize = pageSize;

  String? _pageToken;
  String? get pageToken => _$this._pageToken;
  set pageToken(String? pageToken) => _$this._pageToken = pageToken;

  String? _taskId;
  String? get taskId => _$this._taskId;
  set taskId(String? taskId) => _$this._taskId = taskId;

  String? _tenant;
  String? get tenant => _$this._tenant;
  set tenant(String? tenant) => _$this._tenant = tenant;

  ListTaskPushNotificationConfigsRequestBuilder() {
    ListTaskPushNotificationConfigsRequest._defaults(this);
  }

  ListTaskPushNotificationConfigsRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _pageSize = $v.pageSize;
      _pageToken = $v.pageToken;
      _taskId = $v.taskId;
      _tenant = $v.tenant;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ListTaskPushNotificationConfigsRequest other) {
    _$v = other as _$ListTaskPushNotificationConfigsRequest;
  }

  @override
  void update(
      void Function(ListTaskPushNotificationConfigsRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ListTaskPushNotificationConfigsRequest build() => _build();

  _$ListTaskPushNotificationConfigsRequest _build() {
    final _$result = _$v ??
        _$ListTaskPushNotificationConfigsRequest._(
          pageSize: pageSize,
          pageToken: pageToken,
          taskId: BuiltValueNullFieldError.checkNotNull(
              taskId, r'ListTaskPushNotificationConfigsRequest', 'taskId'),
          tenant: tenant,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
