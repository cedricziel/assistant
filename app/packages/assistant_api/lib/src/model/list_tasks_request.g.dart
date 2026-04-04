// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'list_tasks_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ListTasksRequest extends ListTasksRequest {
  @override
  final String? contextId;
  @override
  final int? historyLength;
  @override
  final bool? includeArtifacts;
  @override
  final int? pageSize;
  @override
  final String? pageToken;
  @override
  final TaskState? status;
  @override
  final DateTime? statusTimestampAfter;
  @override
  final String? tenant;

  factory _$ListTasksRequest(
          [void Function(ListTasksRequestBuilder)? updates]) =>
      (ListTasksRequestBuilder()..update(updates))._build();

  _$ListTasksRequest._(
      {this.contextId,
      this.historyLength,
      this.includeArtifacts,
      this.pageSize,
      this.pageToken,
      this.status,
      this.statusTimestampAfter,
      this.tenant})
      : super._();
  @override
  ListTasksRequest rebuild(void Function(ListTasksRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ListTasksRequestBuilder toBuilder() =>
      ListTasksRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ListTasksRequest &&
        contextId == other.contextId &&
        historyLength == other.historyLength &&
        includeArtifacts == other.includeArtifacts &&
        pageSize == other.pageSize &&
        pageToken == other.pageToken &&
        status == other.status &&
        statusTimestampAfter == other.statusTimestampAfter &&
        tenant == other.tenant;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, contextId.hashCode);
    _$hash = $jc(_$hash, historyLength.hashCode);
    _$hash = $jc(_$hash, includeArtifacts.hashCode);
    _$hash = $jc(_$hash, pageSize.hashCode);
    _$hash = $jc(_$hash, pageToken.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, statusTimestampAfter.hashCode);
    _$hash = $jc(_$hash, tenant.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ListTasksRequest')
          ..add('contextId', contextId)
          ..add('historyLength', historyLength)
          ..add('includeArtifacts', includeArtifacts)
          ..add('pageSize', pageSize)
          ..add('pageToken', pageToken)
          ..add('status', status)
          ..add('statusTimestampAfter', statusTimestampAfter)
          ..add('tenant', tenant))
        .toString();
  }
}

class ListTasksRequestBuilder
    implements Builder<ListTasksRequest, ListTasksRequestBuilder> {
  _$ListTasksRequest? _$v;

  String? _contextId;
  String? get contextId => _$this._contextId;
  set contextId(String? contextId) => _$this._contextId = contextId;

  int? _historyLength;
  int? get historyLength => _$this._historyLength;
  set historyLength(int? historyLength) =>
      _$this._historyLength = historyLength;

  bool? _includeArtifacts;
  bool? get includeArtifacts => _$this._includeArtifacts;
  set includeArtifacts(bool? includeArtifacts) =>
      _$this._includeArtifacts = includeArtifacts;

  int? _pageSize;
  int? get pageSize => _$this._pageSize;
  set pageSize(int? pageSize) => _$this._pageSize = pageSize;

  String? _pageToken;
  String? get pageToken => _$this._pageToken;
  set pageToken(String? pageToken) => _$this._pageToken = pageToken;

  TaskState? _status;
  TaskState? get status => _$this._status;
  set status(TaskState? status) => _$this._status = status;

  DateTime? _statusTimestampAfter;
  DateTime? get statusTimestampAfter => _$this._statusTimestampAfter;
  set statusTimestampAfter(DateTime? statusTimestampAfter) =>
      _$this._statusTimestampAfter = statusTimestampAfter;

  String? _tenant;
  String? get tenant => _$this._tenant;
  set tenant(String? tenant) => _$this._tenant = tenant;

  ListTasksRequestBuilder() {
    ListTasksRequest._defaults(this);
  }

  ListTasksRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _contextId = $v.contextId;
      _historyLength = $v.historyLength;
      _includeArtifacts = $v.includeArtifacts;
      _pageSize = $v.pageSize;
      _pageToken = $v.pageToken;
      _status = $v.status;
      _statusTimestampAfter = $v.statusTimestampAfter;
      _tenant = $v.tenant;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ListTasksRequest other) {
    _$v = other as _$ListTasksRequest;
  }

  @override
  void update(void Function(ListTasksRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ListTasksRequest build() => _build();

  _$ListTasksRequest _build() {
    final _$result = _$v ??
        _$ListTasksRequest._(
          contextId: contextId,
          historyLength: historyLength,
          includeArtifacts: includeArtifacts,
          pageSize: pageSize,
          pageToken: pageToken,
          status: status,
          statusTimestampAfter: statusTimestampAfter,
          tenant: tenant,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
