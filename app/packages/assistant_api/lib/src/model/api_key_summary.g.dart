// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'api_key_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApiKeySummary extends ApiKeySummary {
  @override
  final String createdAt;
  @override
  final String? expiresAt;
  @override
  final String id;
  @override
  final String keyPrefix;
  @override
  final String name;
  @override
  final BuiltList<String> scopes;

  factory _$ApiKeySummary([void Function(ApiKeySummaryBuilder)? updates]) =>
      (ApiKeySummaryBuilder()..update(updates))._build();

  _$ApiKeySummary._(
      {required this.createdAt,
      this.expiresAt,
      required this.id,
      required this.keyPrefix,
      required this.name,
      required this.scopes})
      : super._();
  @override
  ApiKeySummary rebuild(void Function(ApiKeySummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ApiKeySummaryBuilder toBuilder() => ApiKeySummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApiKeySummary &&
        createdAt == other.createdAt &&
        expiresAt == other.expiresAt &&
        id == other.id &&
        keyPrefix == other.keyPrefix &&
        name == other.name &&
        scopes == other.scopes;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, expiresAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, keyPrefix.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, scopes.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApiKeySummary')
          ..add('createdAt', createdAt)
          ..add('expiresAt', expiresAt)
          ..add('id', id)
          ..add('keyPrefix', keyPrefix)
          ..add('name', name)
          ..add('scopes', scopes))
        .toString();
  }
}

class ApiKeySummaryBuilder
    implements Builder<ApiKeySummary, ApiKeySummaryBuilder> {
  _$ApiKeySummary? _$v;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _expiresAt;
  String? get expiresAt => _$this._expiresAt;
  set expiresAt(String? expiresAt) => _$this._expiresAt = expiresAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _keyPrefix;
  String? get keyPrefix => _$this._keyPrefix;
  set keyPrefix(String? keyPrefix) => _$this._keyPrefix = keyPrefix;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  ListBuilder<String>? _scopes;
  ListBuilder<String> get scopes => _$this._scopes ??= ListBuilder<String>();
  set scopes(ListBuilder<String>? scopes) => _$this._scopes = scopes;

  ApiKeySummaryBuilder() {
    ApiKeySummary._defaults(this);
  }

  ApiKeySummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _expiresAt = $v.expiresAt;
      _id = $v.id;
      _keyPrefix = $v.keyPrefix;
      _name = $v.name;
      _scopes = $v.scopes.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApiKeySummary other) {
    _$v = other as _$ApiKeySummary;
  }

  @override
  void update(void Function(ApiKeySummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApiKeySummary build() => _build();

  _$ApiKeySummary _build() {
    _$ApiKeySummary _$result;
    try {
      _$result = _$v ??
          _$ApiKeySummary._(
            createdAt: BuiltValueNullFieldError.checkNotNull(
                createdAt, r'ApiKeySummary', 'createdAt'),
            expiresAt: expiresAt,
            id: BuiltValueNullFieldError.checkNotNull(
                id, r'ApiKeySummary', 'id'),
            keyPrefix: BuiltValueNullFieldError.checkNotNull(
                keyPrefix, r'ApiKeySummary', 'keyPrefix'),
            name: BuiltValueNullFieldError.checkNotNull(
                name, r'ApiKeySummary', 'name'),
            scopes: scopes.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'scopes';
        scopes.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ApiKeySummary', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
