// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'get_task_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$GetTaskRequest extends GetTaskRequest {
  @override
  final int? historyLength;
  @override
  final String id;
  @override
  final String? tenant;

  factory _$GetTaskRequest([void Function(GetTaskRequestBuilder)? updates]) =>
      (GetTaskRequestBuilder()..update(updates))._build();

  _$GetTaskRequest._({this.historyLength, required this.id, this.tenant})
      : super._();
  @override
  GetTaskRequest rebuild(void Function(GetTaskRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  GetTaskRequestBuilder toBuilder() => GetTaskRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is GetTaskRequest &&
        historyLength == other.historyLength &&
        id == other.id &&
        tenant == other.tenant;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, historyLength.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, tenant.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'GetTaskRequest')
          ..add('historyLength', historyLength)
          ..add('id', id)
          ..add('tenant', tenant))
        .toString();
  }
}

class GetTaskRequestBuilder
    implements Builder<GetTaskRequest, GetTaskRequestBuilder> {
  _$GetTaskRequest? _$v;

  int? _historyLength;
  int? get historyLength => _$this._historyLength;
  set historyLength(int? historyLength) =>
      _$this._historyLength = historyLength;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _tenant;
  String? get tenant => _$this._tenant;
  set tenant(String? tenant) => _$this._tenant = tenant;

  GetTaskRequestBuilder() {
    GetTaskRequest._defaults(this);
  }

  GetTaskRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _historyLength = $v.historyLength;
      _id = $v.id;
      _tenant = $v.tenant;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(GetTaskRequest other) {
    _$v = other as _$GetTaskRequest;
  }

  @override
  void update(void Function(GetTaskRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  GetTaskRequest build() => _build();

  _$GetTaskRequest _build() {
    final _$result = _$v ??
        _$GetTaskRequest._(
          historyLength: historyLength,
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'GetTaskRequest', 'id'),
          tenant: tenant,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
