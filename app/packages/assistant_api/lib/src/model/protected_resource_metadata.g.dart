// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'protected_resource_metadata.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ProtectedResourceMetadata extends ProtectedResourceMetadata {
  @override
  final BuiltList<String> authorizationServers;
  @override
  final BuiltList<String> bearerMethodsSupported;
  @override
  final String resource;
  @override
  final BuiltList<String> scopesSupported;

  factory _$ProtectedResourceMetadata(
          [void Function(ProtectedResourceMetadataBuilder)? updates]) =>
      (ProtectedResourceMetadataBuilder()..update(updates))._build();

  _$ProtectedResourceMetadata._(
      {required this.authorizationServers,
      required this.bearerMethodsSupported,
      required this.resource,
      required this.scopesSupported})
      : super._();
  @override
  ProtectedResourceMetadata rebuild(
          void Function(ProtectedResourceMetadataBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ProtectedResourceMetadataBuilder toBuilder() =>
      ProtectedResourceMetadataBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ProtectedResourceMetadata &&
        authorizationServers == other.authorizationServers &&
        bearerMethodsSupported == other.bearerMethodsSupported &&
        resource == other.resource &&
        scopesSupported == other.scopesSupported;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, authorizationServers.hashCode);
    _$hash = $jc(_$hash, bearerMethodsSupported.hashCode);
    _$hash = $jc(_$hash, resource.hashCode);
    _$hash = $jc(_$hash, scopesSupported.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ProtectedResourceMetadata')
          ..add('authorizationServers', authorizationServers)
          ..add('bearerMethodsSupported', bearerMethodsSupported)
          ..add('resource', resource)
          ..add('scopesSupported', scopesSupported))
        .toString();
  }
}

class ProtectedResourceMetadataBuilder
    implements
        Builder<ProtectedResourceMetadata, ProtectedResourceMetadataBuilder> {
  _$ProtectedResourceMetadata? _$v;

  ListBuilder<String>? _authorizationServers;
  ListBuilder<String> get authorizationServers =>
      _$this._authorizationServers ??= ListBuilder<String>();
  set authorizationServers(ListBuilder<String>? authorizationServers) =>
      _$this._authorizationServers = authorizationServers;

  ListBuilder<String>? _bearerMethodsSupported;
  ListBuilder<String> get bearerMethodsSupported =>
      _$this._bearerMethodsSupported ??= ListBuilder<String>();
  set bearerMethodsSupported(ListBuilder<String>? bearerMethodsSupported) =>
      _$this._bearerMethodsSupported = bearerMethodsSupported;

  String? _resource;
  String? get resource => _$this._resource;
  set resource(String? resource) => _$this._resource = resource;

  ListBuilder<String>? _scopesSupported;
  ListBuilder<String> get scopesSupported =>
      _$this._scopesSupported ??= ListBuilder<String>();
  set scopesSupported(ListBuilder<String>? scopesSupported) =>
      _$this._scopesSupported = scopesSupported;

  ProtectedResourceMetadataBuilder() {
    ProtectedResourceMetadata._defaults(this);
  }

  ProtectedResourceMetadataBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authorizationServers = $v.authorizationServers.toBuilder();
      _bearerMethodsSupported = $v.bearerMethodsSupported.toBuilder();
      _resource = $v.resource;
      _scopesSupported = $v.scopesSupported.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ProtectedResourceMetadata other) {
    _$v = other as _$ProtectedResourceMetadata;
  }

  @override
  void update(void Function(ProtectedResourceMetadataBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ProtectedResourceMetadata build() => _build();

  _$ProtectedResourceMetadata _build() {
    _$ProtectedResourceMetadata _$result;
    try {
      _$result = _$v ??
          _$ProtectedResourceMetadata._(
            authorizationServers: authorizationServers.build(),
            bearerMethodsSupported: bearerMethodsSupported.build(),
            resource: BuiltValueNullFieldError.checkNotNull(
                resource, r'ProtectedResourceMetadata', 'resource'),
            scopesSupported: scopesSupported.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'authorizationServers';
        authorizationServers.build();
        _$failedField = 'bearerMethodsSupported';
        bearerMethodsSupported.build();

        _$failedField = 'scopesSupported';
        scopesSupported.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ProtectedResourceMetadata', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
