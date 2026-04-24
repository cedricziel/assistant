// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_api_key_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateApiKeyResponse extends CreateApiKeyResponse {
  @override
  final String id;
  @override
  final String keyPrefix;
  @override
  final String name;
  @override
  final String plaintext;
  @override
  final BuiltList<String> scopes;

  factory _$CreateApiKeyResponse(
          [void Function(CreateApiKeyResponseBuilder)? updates]) =>
      (CreateApiKeyResponseBuilder()..update(updates))._build();

  _$CreateApiKeyResponse._(
      {required this.id,
      required this.keyPrefix,
      required this.name,
      required this.plaintext,
      required this.scopes})
      : super._();
  @override
  CreateApiKeyResponse rebuild(
          void Function(CreateApiKeyResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateApiKeyResponseBuilder toBuilder() =>
      CreateApiKeyResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateApiKeyResponse &&
        id == other.id &&
        keyPrefix == other.keyPrefix &&
        name == other.name &&
        plaintext == other.plaintext &&
        scopes == other.scopes;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, keyPrefix.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, plaintext.hashCode);
    _$hash = $jc(_$hash, scopes.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateApiKeyResponse')
          ..add('id', id)
          ..add('keyPrefix', keyPrefix)
          ..add('name', name)
          ..add('plaintext', plaintext)
          ..add('scopes', scopes))
        .toString();
  }
}

class CreateApiKeyResponseBuilder
    implements Builder<CreateApiKeyResponse, CreateApiKeyResponseBuilder> {
  _$CreateApiKeyResponse? _$v;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _keyPrefix;
  String? get keyPrefix => _$this._keyPrefix;
  set keyPrefix(String? keyPrefix) => _$this._keyPrefix = keyPrefix;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _plaintext;
  String? get plaintext => _$this._plaintext;
  set plaintext(String? plaintext) => _$this._plaintext = plaintext;

  ListBuilder<String>? _scopes;
  ListBuilder<String> get scopes => _$this._scopes ??= ListBuilder<String>();
  set scopes(ListBuilder<String>? scopes) => _$this._scopes = scopes;

  CreateApiKeyResponseBuilder() {
    CreateApiKeyResponse._defaults(this);
  }

  CreateApiKeyResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _id = $v.id;
      _keyPrefix = $v.keyPrefix;
      _name = $v.name;
      _plaintext = $v.plaintext;
      _scopes = $v.scopes.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateApiKeyResponse other) {
    _$v = other as _$CreateApiKeyResponse;
  }

  @override
  void update(void Function(CreateApiKeyResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateApiKeyResponse build() => _build();

  _$CreateApiKeyResponse _build() {
    _$CreateApiKeyResponse _$result;
    try {
      _$result = _$v ??
          _$CreateApiKeyResponse._(
            id: BuiltValueNullFieldError.checkNotNull(
                id, r'CreateApiKeyResponse', 'id'),
            keyPrefix: BuiltValueNullFieldError.checkNotNull(
                keyPrefix, r'CreateApiKeyResponse', 'keyPrefix'),
            name: BuiltValueNullFieldError.checkNotNull(
                name, r'CreateApiKeyResponse', 'name'),
            plaintext: BuiltValueNullFieldError.checkNotNull(
                plaintext, r'CreateApiKeyResponse', 'plaintext'),
            scopes: scopes.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'scopes';
        scopes.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'CreateApiKeyResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
