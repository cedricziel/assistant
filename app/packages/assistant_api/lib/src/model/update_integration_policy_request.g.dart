// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_integration_policy_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateIntegrationPolicyRequest extends UpdateIntegrationPolicyRequest {
  @override
  final bool allowCustomUrls;
  @override
  final bool allowUserIntegrations;
  @override
  final BuiltList<String>? allowedCatalogIds;
  @override
  final CatalogMode catalogMode;

  factory _$UpdateIntegrationPolicyRequest(
          [void Function(UpdateIntegrationPolicyRequestBuilder)? updates]) =>
      (UpdateIntegrationPolicyRequestBuilder()..update(updates))._build();

  _$UpdateIntegrationPolicyRequest._(
      {required this.allowCustomUrls,
      required this.allowUserIntegrations,
      this.allowedCatalogIds,
      required this.catalogMode})
      : super._();
  @override
  UpdateIntegrationPolicyRequest rebuild(
          void Function(UpdateIntegrationPolicyRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UpdateIntegrationPolicyRequestBuilder toBuilder() =>
      UpdateIntegrationPolicyRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateIntegrationPolicyRequest &&
        allowCustomUrls == other.allowCustomUrls &&
        allowUserIntegrations == other.allowUserIntegrations &&
        allowedCatalogIds == other.allowedCatalogIds &&
        catalogMode == other.catalogMode;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, allowCustomUrls.hashCode);
    _$hash = $jc(_$hash, allowUserIntegrations.hashCode);
    _$hash = $jc(_$hash, allowedCatalogIds.hashCode);
    _$hash = $jc(_$hash, catalogMode.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdateIntegrationPolicyRequest')
          ..add('allowCustomUrls', allowCustomUrls)
          ..add('allowUserIntegrations', allowUserIntegrations)
          ..add('allowedCatalogIds', allowedCatalogIds)
          ..add('catalogMode', catalogMode))
        .toString();
  }
}

class UpdateIntegrationPolicyRequestBuilder
    implements
        Builder<UpdateIntegrationPolicyRequest,
            UpdateIntegrationPolicyRequestBuilder> {
  _$UpdateIntegrationPolicyRequest? _$v;

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

  UpdateIntegrationPolicyRequestBuilder() {
    UpdateIntegrationPolicyRequest._defaults(this);
  }

  UpdateIntegrationPolicyRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _allowCustomUrls = $v.allowCustomUrls;
      _allowUserIntegrations = $v.allowUserIntegrations;
      _allowedCatalogIds = $v.allowedCatalogIds?.toBuilder();
      _catalogMode = $v.catalogMode;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateIntegrationPolicyRequest other) {
    _$v = other as _$UpdateIntegrationPolicyRequest;
  }

  @override
  void update(void Function(UpdateIntegrationPolicyRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateIntegrationPolicyRequest build() => _build();

  _$UpdateIntegrationPolicyRequest _build() {
    _$UpdateIntegrationPolicyRequest _$result;
    try {
      _$result = _$v ??
          _$UpdateIntegrationPolicyRequest._(
            allowCustomUrls: BuiltValueNullFieldError.checkNotNull(
                allowCustomUrls,
                r'UpdateIntegrationPolicyRequest',
                'allowCustomUrls'),
            allowUserIntegrations: BuiltValueNullFieldError.checkNotNull(
                allowUserIntegrations,
                r'UpdateIntegrationPolicyRequest',
                'allowUserIntegrations'),
            allowedCatalogIds: _allowedCatalogIds?.build(),
            catalogMode: BuiltValueNullFieldError.checkNotNull(
                catalogMode, r'UpdateIntegrationPolicyRequest', 'catalogMode'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'allowedCatalogIds';
        _allowedCatalogIds?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'UpdateIntegrationPolicyRequest', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
