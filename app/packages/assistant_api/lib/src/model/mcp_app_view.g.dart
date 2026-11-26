// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'mcp_app_view.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$McpAppView extends McpAppView {
  @override
  final McpAppCspView csp;
  @override
  final String html;
  @override
  final McpAppPermissionsView permissions;
  @override
  final bool? prefersBorder;
  @override
  final String resourceUri;
  @override
  final String server;

  factory _$McpAppView([void Function(McpAppViewBuilder)? updates]) =>
      (McpAppViewBuilder()..update(updates))._build();

  _$McpAppView._(
      {required this.csp,
      required this.html,
      required this.permissions,
      this.prefersBorder,
      required this.resourceUri,
      required this.server})
      : super._();
  @override
  McpAppView rebuild(void Function(McpAppViewBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  McpAppViewBuilder toBuilder() => McpAppViewBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is McpAppView &&
        csp == other.csp &&
        html == other.html &&
        permissions == other.permissions &&
        prefersBorder == other.prefersBorder &&
        resourceUri == other.resourceUri &&
        server == other.server;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, csp.hashCode);
    _$hash = $jc(_$hash, html.hashCode);
    _$hash = $jc(_$hash, permissions.hashCode);
    _$hash = $jc(_$hash, prefersBorder.hashCode);
    _$hash = $jc(_$hash, resourceUri.hashCode);
    _$hash = $jc(_$hash, server.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'McpAppView')
          ..add('csp', csp)
          ..add('html', html)
          ..add('permissions', permissions)
          ..add('prefersBorder', prefersBorder)
          ..add('resourceUri', resourceUri)
          ..add('server', server))
        .toString();
  }
}

class McpAppViewBuilder implements Builder<McpAppView, McpAppViewBuilder> {
  _$McpAppView? _$v;

  McpAppCspViewBuilder? _csp;
  McpAppCspViewBuilder get csp => _$this._csp ??= McpAppCspViewBuilder();
  set csp(McpAppCspViewBuilder? csp) => _$this._csp = csp;

  String? _html;
  String? get html => _$this._html;
  set html(String? html) => _$this._html = html;

  McpAppPermissionsViewBuilder? _permissions;
  McpAppPermissionsViewBuilder get permissions =>
      _$this._permissions ??= McpAppPermissionsViewBuilder();
  set permissions(McpAppPermissionsViewBuilder? permissions) =>
      _$this._permissions = permissions;

  bool? _prefersBorder;
  bool? get prefersBorder => _$this._prefersBorder;
  set prefersBorder(bool? prefersBorder) =>
      _$this._prefersBorder = prefersBorder;

  String? _resourceUri;
  String? get resourceUri => _$this._resourceUri;
  set resourceUri(String? resourceUri) => _$this._resourceUri = resourceUri;

  String? _server;
  String? get server => _$this._server;
  set server(String? server) => _$this._server = server;

  McpAppViewBuilder() {
    McpAppView._defaults(this);
  }

  McpAppViewBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _csp = $v.csp.toBuilder();
      _html = $v.html;
      _permissions = $v.permissions.toBuilder();
      _prefersBorder = $v.prefersBorder;
      _resourceUri = $v.resourceUri;
      _server = $v.server;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(McpAppView other) {
    _$v = other as _$McpAppView;
  }

  @override
  void update(void Function(McpAppViewBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  McpAppView build() => _build();

  _$McpAppView _build() {
    _$McpAppView _$result;
    try {
      _$result = _$v ??
          _$McpAppView._(
            csp: csp.build(),
            html: BuiltValueNullFieldError.checkNotNull(
                html, r'McpAppView', 'html'),
            permissions: permissions.build(),
            prefersBorder: prefersBorder,
            resourceUri: BuiltValueNullFieldError.checkNotNull(
                resourceUri, r'McpAppView', 'resourceUri'),
            server: BuiltValueNullFieldError.checkNotNull(
                server, r'McpAppView', 'server'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'csp';
        csp.build();

        _$failedField = 'permissions';
        permissions.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'McpAppView', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
