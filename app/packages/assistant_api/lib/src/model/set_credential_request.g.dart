// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'set_credential_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SetCredentialRequest extends SetCredentialRequest {
  @override
  final String? headerName;
  @override
  final String value;

  factory _$SetCredentialRequest(
          [void Function(SetCredentialRequestBuilder)? updates]) =>
      (SetCredentialRequestBuilder()..update(updates))._build();

  _$SetCredentialRequest._({this.headerName, required this.value}) : super._();
  @override
  SetCredentialRequest rebuild(
          void Function(SetCredentialRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SetCredentialRequestBuilder toBuilder() =>
      SetCredentialRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SetCredentialRequest &&
        headerName == other.headerName &&
        value == other.value;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, headerName.hashCode);
    _$hash = $jc(_$hash, value.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SetCredentialRequest')
          ..add('headerName', headerName)
          ..add('value', value))
        .toString();
  }
}

class SetCredentialRequestBuilder
    implements Builder<SetCredentialRequest, SetCredentialRequestBuilder> {
  _$SetCredentialRequest? _$v;

  String? _headerName;
  String? get headerName => _$this._headerName;
  set headerName(String? headerName) => _$this._headerName = headerName;

  String? _value;
  String? get value => _$this._value;
  set value(String? value) => _$this._value = value;

  SetCredentialRequestBuilder() {
    SetCredentialRequest._defaults(this);
  }

  SetCredentialRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _headerName = $v.headerName;
      _value = $v.value;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SetCredentialRequest other) {
    _$v = other as _$SetCredentialRequest;
  }

  @override
  void update(void Function(SetCredentialRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SetCredentialRequest build() => _build();

  _$SetCredentialRequest _build() {
    final _$result = _$v ??
        _$SetCredentialRequest._(
          headerName: headerName,
          value: BuiltValueNullFieldError.checkNotNull(
              value, r'SetCredentialRequest', 'value'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
