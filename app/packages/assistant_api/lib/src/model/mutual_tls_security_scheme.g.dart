// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'mutual_tls_security_scheme.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$MutualTlsSecurityScheme extends MutualTlsSecurityScheme {
  @override
  final String? description;

  factory _$MutualTlsSecurityScheme(
          [void Function(MutualTlsSecuritySchemeBuilder)? updates]) =>
      (MutualTlsSecuritySchemeBuilder()..update(updates))._build();

  _$MutualTlsSecurityScheme._({this.description}) : super._();
  @override
  MutualTlsSecurityScheme rebuild(
          void Function(MutualTlsSecuritySchemeBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  MutualTlsSecuritySchemeBuilder toBuilder() =>
      MutualTlsSecuritySchemeBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is MutualTlsSecurityScheme && description == other.description;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'MutualTlsSecurityScheme')
          ..add('description', description))
        .toString();
  }
}

class MutualTlsSecuritySchemeBuilder
    implements
        Builder<MutualTlsSecurityScheme, MutualTlsSecuritySchemeBuilder> {
  _$MutualTlsSecurityScheme? _$v;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  MutualTlsSecuritySchemeBuilder() {
    MutualTlsSecurityScheme._defaults(this);
  }

  MutualTlsSecuritySchemeBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(MutualTlsSecurityScheme other) {
    _$v = other as _$MutualTlsSecurityScheme;
  }

  @override
  void update(void Function(MutualTlsSecuritySchemeBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  MutualTlsSecurityScheme build() => _build();

  _$MutualTlsSecurityScheme _build() {
    final _$result = _$v ??
        _$MutualTlsSecurityScheme._(
          description: description,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
