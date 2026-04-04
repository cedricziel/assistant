// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'subscribe_to_task_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SubscribeToTaskRequest extends SubscribeToTaskRequest {
  @override
  final String id;
  @override
  final String? tenant;

  factory _$SubscribeToTaskRequest(
          [void Function(SubscribeToTaskRequestBuilder)? updates]) =>
      (SubscribeToTaskRequestBuilder()..update(updates))._build();

  _$SubscribeToTaskRequest._({required this.id, this.tenant}) : super._();
  @override
  SubscribeToTaskRequest rebuild(
          void Function(SubscribeToTaskRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SubscribeToTaskRequestBuilder toBuilder() =>
      SubscribeToTaskRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SubscribeToTaskRequest &&
        id == other.id &&
        tenant == other.tenant;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, tenant.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SubscribeToTaskRequest')
          ..add('id', id)
          ..add('tenant', tenant))
        .toString();
  }
}

class SubscribeToTaskRequestBuilder
    implements Builder<SubscribeToTaskRequest, SubscribeToTaskRequestBuilder> {
  _$SubscribeToTaskRequest? _$v;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _tenant;
  String? get tenant => _$this._tenant;
  set tenant(String? tenant) => _$this._tenant = tenant;

  SubscribeToTaskRequestBuilder() {
    SubscribeToTaskRequest._defaults(this);
  }

  SubscribeToTaskRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _id = $v.id;
      _tenant = $v.tenant;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SubscribeToTaskRequest other) {
    _$v = other as _$SubscribeToTaskRequest;
  }

  @override
  void update(void Function(SubscribeToTaskRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SubscribeToTaskRequest build() => _build();

  _$SubscribeToTaskRequest _build() {
    final _$result = _$v ??
        _$SubscribeToTaskRequest._(
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'SubscribeToTaskRequest', 'id'),
          tenant: tenant,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
