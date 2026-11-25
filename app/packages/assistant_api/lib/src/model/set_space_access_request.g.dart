// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'set_space_access_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SetSpaceAccessRequest extends SetSpaceAccessRequest {
  @override
  final SpaceAccessMode mode;

  factory _$SetSpaceAccessRequest(
          [void Function(SetSpaceAccessRequestBuilder)? updates]) =>
      (SetSpaceAccessRequestBuilder()..update(updates))._build();

  _$SetSpaceAccessRequest._({required this.mode}) : super._();
  @override
  SetSpaceAccessRequest rebuild(
          void Function(SetSpaceAccessRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SetSpaceAccessRequestBuilder toBuilder() =>
      SetSpaceAccessRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SetSpaceAccessRequest && mode == other.mode;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, mode.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SetSpaceAccessRequest')
          ..add('mode', mode))
        .toString();
  }
}

class SetSpaceAccessRequestBuilder
    implements Builder<SetSpaceAccessRequest, SetSpaceAccessRequestBuilder> {
  _$SetSpaceAccessRequest? _$v;

  SpaceAccessMode? _mode;
  SpaceAccessMode? get mode => _$this._mode;
  set mode(SpaceAccessMode? mode) => _$this._mode = mode;

  SetSpaceAccessRequestBuilder() {
    SetSpaceAccessRequest._defaults(this);
  }

  SetSpaceAccessRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _mode = $v.mode;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SetSpaceAccessRequest other) {
    _$v = other as _$SetSpaceAccessRequest;
  }

  @override
  void update(void Function(SetSpaceAccessRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SetSpaceAccessRequest build() => _build();

  _$SetSpaceAccessRequest _build() {
    final _$result = _$v ??
        _$SetSpaceAccessRequest._(
          mode: BuiltValueNullFieldError.checkNotNull(
              mode, r'SetSpaceAccessRequest', 'mode'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
