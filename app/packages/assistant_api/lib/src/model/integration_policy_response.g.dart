// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'integration_policy_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$IntegrationPolicyResponse extends IntegrationPolicyResponse {
  @override
  final bool allowCustomUrls;
  @override
  final bool allowUserIntegrations;
  @override
  final BuiltList<String> allowedCatalogIds;
  @override
  final CatalogMode catalogMode;
  @override
  final DateTime? updatedAt;

  factory _$IntegrationPolicyResponse(
          [void Function(IntegrationPolicyResponseBuilder)? updates]) =>
      (IntegrationPolicyResponseBuilder()..update(updates))._build();

  _$IntegrationPolicyResponse._(
      {required this.allowCustomUrls,
      required this.allowUserIntegrations,
      required this.allowedCatalogIds,
      required this.catalogMode,
      this.updatedAt})
      : super._();
  @override
  IntegrationPolicyResponse rebuild(
          void Function(IntegrationPolicyResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  IntegrationPolicyResponseBuilder toBuilder() =>
      IntegrationPolicyResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is IntegrationPolicyResponse &&
        allowCustomUrls == other.allowCustomUrls &&
        allowUserIntegrations == other.allowUserIntegrations &&
        allowedCatalogIds == other.allowedCatalogIds &&
        catalogMode == other.catalogMode &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, allowCustomUrls.hashCode);
    _$hash = $jc(_$hash, allowUserIntegrations.hashCode);
    _$hash = $jc(_$hash, allowedCatalogIds.hashCode);
    _$hash = $jc(_$hash, catalogMode.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'IntegrationPolicyResponse')
          ..add('allowCustomUrls', allowCustomUrls)
          ..add('allowUserIntegrations', allowUserIntegrations)
          ..add('allowedCatalogIds', allowedCatalogIds)
          ..add('catalogMode', catalogMode)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class IntegrationPolicyResponseBuilder
    implements
        Builder<IntegrationPolicyResponse, IntegrationPolicyResponseBuilder> {
  _$IntegrationPolicyResponse? _$v;

  bool? _allowCustomUrls;
  bool? get allowCustomUrls => _$this._allowCustomUrls;
  set allowCustomUrls(bool? allowCustomUrls) =>
      _$this._allowCustomUrls = allowCustomUrls;

  bool? _allowUserIntegrations;
  bool? get allowUserIntegrations => _$this._allowUserIntegrations;
  set allowUserIntegrations(bool? allowUserIntegrations) =>
      _$this._allowUserIntegrations = allowUserIntegrations;

  ListBuilder<String>? _allowedCatalogIds;
  ListBuilder<String> get allowedCatalogIds =>
      _$this._allowedCatalogIds ??= ListBuilder<String>();
  set allowedCatalogIds(ListBuilder<String>? allowedCatalogIds) =>
      _$this._allowedCatalogIds = allowedCatalogIds;

  CatalogMode? _catalogMode;
  CatalogMode? get catalogMode => _$this._catalogMode;
  set catalogMode(CatalogMode? catalogMode) =>
      _$this._catalogMode = catalogMode;

  DateTime? _updatedAt;
  DateTime? get updatedAt => _$this._updatedAt;
  set updatedAt(DateTime? updatedAt) => _$this._updatedAt = updatedAt;

  IntegrationPolicyResponseBuilder() {
    IntegrationPolicyResponse._defaults(this);
  }

  IntegrationPolicyResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _allowCustomUrls = $v.allowCustomUrls;
      _allowUserIntegrations = $v.allowUserIntegrations;
      _allowedCatalogIds = $v.allowedCatalogIds.toBuilder();
      _catalogMode = $v.catalogMode;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(IntegrationPolicyResponse other) {
    _$v = other as _$IntegrationPolicyResponse;
  }

  @override
  void update(void Function(IntegrationPolicyResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  IntegrationPolicyResponse build() => _build();

  _$IntegrationPolicyResponse _build() {
    _$IntegrationPolicyResponse _$result;
    try {
      _$result = _$v ??
          _$IntegrationPolicyResponse._(
            allowCustomUrls: BuiltValueNullFieldError.checkNotNull(
                allowCustomUrls,
                r'IntegrationPolicyResponse',
                'allowCustomUrls'),
            allowUserIntegrations: BuiltValueNullFieldError.checkNotNull(
                allowUserIntegrations,
                r'IntegrationPolicyResponse',
                'allowUserIntegrations'),
            allowedCatalogIds: allowedCatalogIds.build(),
            catalogMode: BuiltValueNullFieldError.checkNotNull(
                catalogMode, r'IntegrationPolicyResponse', 'catalogMode'),
            updatedAt: updatedAt,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'allowedCatalogIds';
        allowedCatalogIds.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'IntegrationPolicyResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
