// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'o_auth2_security_scheme.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$OAuth2SecurityScheme extends OAuth2SecurityScheme {
  @override
  final String? description;
  @override
  final OAuthFlows flows;
  @override
  final String? oauth2MetadataUrl;

  factory _$OAuth2SecurityScheme(
          [void Function(OAuth2SecuritySchemeBuilder)? updates]) =>
      (OAuth2SecuritySchemeBuilder()..update(updates))._build();

  _$OAuth2SecurityScheme._(
      {this.description, required this.flows, this.oauth2MetadataUrl})
      : super._();
  @override
  OAuth2SecurityScheme rebuild(
          void Function(OAuth2SecuritySchemeBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  OAuth2SecuritySchemeBuilder toBuilder() =>
      OAuth2SecuritySchemeBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is OAuth2SecurityScheme &&
        description == other.description &&
        flows == other.flows &&
        oauth2MetadataUrl == other.oauth2MetadataUrl;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, flows.hashCode);
    _$hash = $jc(_$hash, oauth2MetadataUrl.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'OAuth2SecurityScheme')
          ..add('description', description)
          ..add('flows', flows)
          ..add('oauth2MetadataUrl', oauth2MetadataUrl))
        .toString();
  }
}

class OAuth2SecuritySchemeBuilder
    implements Builder<OAuth2SecurityScheme, OAuth2SecuritySchemeBuilder> {
  _$OAuth2SecurityScheme? _$v;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  OAuthFlowsBuilder? _flows;
  OAuthFlowsBuilder get flows => _$this._flows ??= OAuthFlowsBuilder();
  set flows(OAuthFlowsBuilder? flows) => _$this._flows = flows;

  String? _oauth2MetadataUrl;
  String? get oauth2MetadataUrl => _$this._oauth2MetadataUrl;
  set oauth2MetadataUrl(String? oauth2MetadataUrl) =>
      _$this._oauth2MetadataUrl = oauth2MetadataUrl;

  OAuth2SecuritySchemeBuilder() {
    OAuth2SecurityScheme._defaults(this);
  }

  OAuth2SecuritySchemeBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _flows = $v.flows.toBuilder();
      _oauth2MetadataUrl = $v.oauth2MetadataUrl;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(OAuth2SecurityScheme other) {
    _$v = other as _$OAuth2SecurityScheme;
  }

  @override
  void update(void Function(OAuth2SecuritySchemeBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  OAuth2SecurityScheme build() => _build();

  _$OAuth2SecurityScheme _build() {
    _$OAuth2SecurityScheme _$result;
    try {
      _$result = _$v ??
          _$OAuth2SecurityScheme._(
            description: description,
            flows: flows.build(),
            oauth2MetadataUrl: oauth2MetadataUrl,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'flows';
        flows.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'OAuth2SecurityScheme', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
