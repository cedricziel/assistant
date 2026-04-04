// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'http_auth_security_scheme.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$HttpAuthSecurityScheme extends HttpAuthSecurityScheme {
  @override
  final String? bearerFormat;
  @override
  final String? description;
  @override
  final String scheme;

  factory _$HttpAuthSecurityScheme(
          [void Function(HttpAuthSecuritySchemeBuilder)? updates]) =>
      (HttpAuthSecuritySchemeBuilder()..update(updates))._build();

  _$HttpAuthSecurityScheme._(
      {this.bearerFormat, this.description, required this.scheme})
      : super._();
  @override
  HttpAuthSecurityScheme rebuild(
          void Function(HttpAuthSecuritySchemeBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  HttpAuthSecuritySchemeBuilder toBuilder() =>
      HttpAuthSecuritySchemeBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is HttpAuthSecurityScheme &&
        bearerFormat == other.bearerFormat &&
        description == other.description &&
        scheme == other.scheme;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, bearerFormat.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, scheme.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'HttpAuthSecurityScheme')
          ..add('bearerFormat', bearerFormat)
          ..add('description', description)
          ..add('scheme', scheme))
        .toString();
  }
}

class HttpAuthSecuritySchemeBuilder
    implements Builder<HttpAuthSecurityScheme, HttpAuthSecuritySchemeBuilder> {
  _$HttpAuthSecurityScheme? _$v;

  String? _bearerFormat;
  String? get bearerFormat => _$this._bearerFormat;
  set bearerFormat(String? bearerFormat) => _$this._bearerFormat = bearerFormat;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _scheme;
  String? get scheme => _$this._scheme;
  set scheme(String? scheme) => _$this._scheme = scheme;

  HttpAuthSecuritySchemeBuilder() {
    HttpAuthSecurityScheme._defaults(this);
  }

  HttpAuthSecuritySchemeBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _bearerFormat = $v.bearerFormat;
      _description = $v.description;
      _scheme = $v.scheme;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(HttpAuthSecurityScheme other) {
    _$v = other as _$HttpAuthSecurityScheme;
  }

  @override
  void update(void Function(HttpAuthSecuritySchemeBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  HttpAuthSecurityScheme build() => _build();

  _$HttpAuthSecurityScheme _build() {
    final _$result = _$v ??
        _$HttpAuthSecurityScheme._(
          bearerFormat: bearerFormat,
          description: description,
          scheme: BuiltValueNullFieldError.checkNotNull(
              scheme, r'HttpAuthSecurityScheme', 'scheme'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
