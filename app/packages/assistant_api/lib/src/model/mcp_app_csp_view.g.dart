// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'mcp_app_csp_view.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$McpAppCspView extends McpAppCspView {
  @override
  final BuiltList<String>? baseUriDomains;
  @override
  final BuiltList<String>? connectDomains;
  @override
  final BuiltList<String>? frameDomains;
  @override
  final BuiltList<String>? resourceDomains;

  factory _$McpAppCspView([void Function(McpAppCspViewBuilder)? updates]) =>
      (McpAppCspViewBuilder()..update(updates))._build();

  _$McpAppCspView._(
      {this.baseUriDomains,
      this.connectDomains,
      this.frameDomains,
      this.resourceDomains})
      : super._();
  @override
  McpAppCspView rebuild(void Function(McpAppCspViewBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  McpAppCspViewBuilder toBuilder() => McpAppCspViewBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is McpAppCspView &&
        baseUriDomains == other.baseUriDomains &&
        connectDomains == other.connectDomains &&
        frameDomains == other.frameDomains &&
        resourceDomains == other.resourceDomains;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, baseUriDomains.hashCode);
    _$hash = $jc(_$hash, connectDomains.hashCode);
    _$hash = $jc(_$hash, frameDomains.hashCode);
    _$hash = $jc(_$hash, resourceDomains.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'McpAppCspView')
          ..add('baseUriDomains', baseUriDomains)
          ..add('connectDomains', connectDomains)
          ..add('frameDomains', frameDomains)
          ..add('resourceDomains', resourceDomains))
        .toString();
  }
}

class McpAppCspViewBuilder
    implements Builder<McpAppCspView, McpAppCspViewBuilder> {
  _$McpAppCspView? _$v;

  ListBuilder<String>? _baseUriDomains;
  ListBuilder<String> get baseUriDomains =>
      _$this._baseUriDomains ??= ListBuilder<String>();
  set baseUriDomains(ListBuilder<String>? baseUriDomains) =>
      _$this._baseUriDomains = baseUriDomains;

  ListBuilder<String>? _connectDomains;
  ListBuilder<String> get connectDomains =>
      _$this._connectDomains ??= ListBuilder<String>();
  set connectDomains(ListBuilder<String>? connectDomains) =>
      _$this._connectDomains = connectDomains;

  ListBuilder<String>? _frameDomains;
  ListBuilder<String> get frameDomains =>
      _$this._frameDomains ??= ListBuilder<String>();
  set frameDomains(ListBuilder<String>? frameDomains) =>
      _$this._frameDomains = frameDomains;

  ListBuilder<String>? _resourceDomains;
  ListBuilder<String> get resourceDomains =>
      _$this._resourceDomains ??= ListBuilder<String>();
  set resourceDomains(ListBuilder<String>? resourceDomains) =>
      _$this._resourceDomains = resourceDomains;

  McpAppCspViewBuilder() {
    McpAppCspView._defaults(this);
  }

  McpAppCspViewBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _baseUriDomains = $v.baseUriDomains?.toBuilder();
      _connectDomains = $v.connectDomains?.toBuilder();
      _frameDomains = $v.frameDomains?.toBuilder();
      _resourceDomains = $v.resourceDomains?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(McpAppCspView other) {
    _$v = other as _$McpAppCspView;
  }

  @override
  void update(void Function(McpAppCspViewBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  McpAppCspView build() => _build();

  _$McpAppCspView _build() {
    _$McpAppCspView _$result;
    try {
      _$result = _$v ??
          _$McpAppCspView._(
            baseUriDomains: _baseUriDomains?.build(),
            connectDomains: _connectDomains?.build(),
            frameDomains: _frameDomains?.build(),
            resourceDomains: _resourceDomains?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'baseUriDomains';
        _baseUriDomains?.build();
        _$failedField = 'connectDomains';
        _connectDomains?.build();
        _$failedField = 'frameDomains';
        _frameDomains?.build();
        _$failedField = 'resourceDomains';
        _resourceDomains?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'McpAppCspView', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
