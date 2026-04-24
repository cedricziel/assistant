// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_api_key_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateApiKeyRequest extends CreateApiKeyRequest {
  @override
  final String name;
  @override
  final BuiltList<String>? scopes;

  factory _$CreateApiKeyRequest(
          [void Function(CreateApiKeyRequestBuilder)? updates]) =>
      (CreateApiKeyRequestBuilder()..update(updates))._build();

  _$CreateApiKeyRequest._({required this.name, this.scopes}) : super._();
  @override
  CreateApiKeyRequest rebuild(
          void Function(CreateApiKeyRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateApiKeyRequestBuilder toBuilder() =>
      CreateApiKeyRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateApiKeyRequest &&
        name == other.name &&
        scopes == other.scopes;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, scopes.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateApiKeyRequest')
          ..add('name', name)
          ..add('scopes', scopes))
        .toString();
  }
}

class CreateApiKeyRequestBuilder
    implements Builder<CreateApiKeyRequest, CreateApiKeyRequestBuilder> {
  _$CreateApiKeyRequest? _$v;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  ListBuilder<String>? _scopes;
  ListBuilder<String> get scopes => _$this._scopes ??= ListBuilder<String>();
  set scopes(ListBuilder<String>? scopes) => _$this._scopes = scopes;

  CreateApiKeyRequestBuilder() {
    CreateApiKeyRequest._defaults(this);
  }

  CreateApiKeyRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _name = $v.name;
      _scopes = $v.scopes?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateApiKeyRequest other) {
    _$v = other as _$CreateApiKeyRequest;
  }

  @override
  void update(void Function(CreateApiKeyRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateApiKeyRequest build() => _build();

  _$CreateApiKeyRequest _build() {
    _$CreateApiKeyRequest _$result;
    try {
      _$result = _$v ??
          _$CreateApiKeyRequest._(
            name: BuiltValueNullFieldError.checkNotNull(
                name, r'CreateApiKeyRequest', 'name'),
            scopes: _scopes?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'scopes';
        _scopes?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'CreateApiKeyRequest', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
