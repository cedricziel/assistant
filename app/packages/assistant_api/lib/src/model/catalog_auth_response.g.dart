// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'catalog_auth_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CatalogAuthResponse extends CatalogAuthResponse {
  @override
  final String? headerName;
  @override
  final CatalogAuthKindResponse kind;
  @override
  final String? placeholder;
  @override
  final String? valuePrefix;

  factory _$CatalogAuthResponse(
          [void Function(CatalogAuthResponseBuilder)? updates]) =>
      (CatalogAuthResponseBuilder()..update(updates))._build();

  _$CatalogAuthResponse._(
      {this.headerName, required this.kind, this.placeholder, this.valuePrefix})
      : super._();
  @override
  CatalogAuthResponse rebuild(
          void Function(CatalogAuthResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CatalogAuthResponseBuilder toBuilder() =>
      CatalogAuthResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CatalogAuthResponse &&
        headerName == other.headerName &&
        kind == other.kind &&
        placeholder == other.placeholder &&
        valuePrefix == other.valuePrefix;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, headerName.hashCode);
    _$hash = $jc(_$hash, kind.hashCode);
    _$hash = $jc(_$hash, placeholder.hashCode);
    _$hash = $jc(_$hash, valuePrefix.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CatalogAuthResponse')
          ..add('headerName', headerName)
          ..add('kind', kind)
          ..add('placeholder', placeholder)
          ..add('valuePrefix', valuePrefix))
        .toString();
  }
}

class CatalogAuthResponseBuilder
    implements Builder<CatalogAuthResponse, CatalogAuthResponseBuilder> {
  _$CatalogAuthResponse? _$v;

  String? _headerName;
  String? get headerName => _$this._headerName;
  set headerName(String? headerName) => _$this._headerName = headerName;

  CatalogAuthKindResponse? _kind;
  CatalogAuthKindResponse? get kind => _$this._kind;
  set kind(CatalogAuthKindResponse? kind) => _$this._kind = kind;

  String? _placeholder;
  String? get placeholder => _$this._placeholder;
  set placeholder(String? placeholder) => _$this._placeholder = placeholder;

  String? _valuePrefix;
  String? get valuePrefix => _$this._valuePrefix;
  set valuePrefix(String? valuePrefix) => _$this._valuePrefix = valuePrefix;

  CatalogAuthResponseBuilder() {
    CatalogAuthResponse._defaults(this);
  }

  CatalogAuthResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _headerName = $v.headerName;
      _kind = $v.kind;
      _placeholder = $v.placeholder;
      _valuePrefix = $v.valuePrefix;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CatalogAuthResponse other) {
    _$v = other as _$CatalogAuthResponse;
  }

  @override
  void update(void Function(CatalogAuthResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CatalogAuthResponse build() => _build();

  _$CatalogAuthResponse _build() {
    final _$result = _$v ??
        _$CatalogAuthResponse._(
          headerName: headerName,
          kind: BuiltValueNullFieldError.checkNotNull(
              kind, r'CatalogAuthResponse', 'kind'),
          placeholder: placeholder,
          valuePrefix: valuePrefix,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
