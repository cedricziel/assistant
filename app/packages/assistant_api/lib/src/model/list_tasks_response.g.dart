// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'list_tasks_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ListTasksResponse extends ListTasksResponse {
  @override
  final String nextPageToken;
  @override
  final int pageSize;
  @override
  final BuiltList<Task> tasks;
  @override
  final int totalSize;

  factory _$ListTasksResponse(
          [void Function(ListTasksResponseBuilder)? updates]) =>
      (ListTasksResponseBuilder()..update(updates))._build();

  _$ListTasksResponse._(
      {required this.nextPageToken,
      required this.pageSize,
      required this.tasks,
      required this.totalSize})
      : super._();
  @override
  ListTasksResponse rebuild(void Function(ListTasksResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ListTasksResponseBuilder toBuilder() =>
      ListTasksResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ListTasksResponse &&
        nextPageToken == other.nextPageToken &&
        pageSize == other.pageSize &&
        tasks == other.tasks &&
        totalSize == other.totalSize;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, nextPageToken.hashCode);
    _$hash = $jc(_$hash, pageSize.hashCode);
    _$hash = $jc(_$hash, tasks.hashCode);
    _$hash = $jc(_$hash, totalSize.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ListTasksResponse')
          ..add('nextPageToken', nextPageToken)
          ..add('pageSize', pageSize)
          ..add('tasks', tasks)
          ..add('totalSize', totalSize))
        .toString();
  }
}

class ListTasksResponseBuilder
    implements Builder<ListTasksResponse, ListTasksResponseBuilder> {
  _$ListTasksResponse? _$v;

  String? _nextPageToken;
  String? get nextPageToken => _$this._nextPageToken;
  set nextPageToken(String? nextPageToken) =>
      _$this._nextPageToken = nextPageToken;

  int? _pageSize;
  int? get pageSize => _$this._pageSize;
  set pageSize(int? pageSize) => _$this._pageSize = pageSize;

  ListBuilder<Task>? _tasks;
  ListBuilder<Task> get tasks => _$this._tasks ??= ListBuilder<Task>();
  set tasks(ListBuilder<Task>? tasks) => _$this._tasks = tasks;

  int? _totalSize;
  int? get totalSize => _$this._totalSize;
  set totalSize(int? totalSize) => _$this._totalSize = totalSize;

  ListTasksResponseBuilder() {
    ListTasksResponse._defaults(this);
  }

  ListTasksResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _nextPageToken = $v.nextPageToken;
      _pageSize = $v.pageSize;
      _tasks = $v.tasks.toBuilder();
      _totalSize = $v.totalSize;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ListTasksResponse other) {
    _$v = other as _$ListTasksResponse;
  }

  @override
  void update(void Function(ListTasksResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ListTasksResponse build() => _build();

  _$ListTasksResponse _build() {
    _$ListTasksResponse _$result;
    try {
      _$result = _$v ??
          _$ListTasksResponse._(
            nextPageToken: BuiltValueNullFieldError.checkNotNull(
                nextPageToken, r'ListTasksResponse', 'nextPageToken'),
            pageSize: BuiltValueNullFieldError.checkNotNull(
                pageSize, r'ListTasksResponse', 'pageSize'),
            tasks: tasks.build(),
            totalSize: BuiltValueNullFieldError.checkNotNull(
                totalSize, r'ListTasksResponse', 'totalSize'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'tasks';
        tasks.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ListTasksResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
