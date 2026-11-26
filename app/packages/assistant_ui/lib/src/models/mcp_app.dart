// -- MCP Apps ----------------------------------------------------------------

/// The interactive UI an external MCP tool declared for its result.
class McpApp {
  const McpApp({
    required this.server,
    required this.resourceUri,
    required this.html,
    this.csp = const McpAppCsp(),
    this.permissions = const McpAppPermissions(),
    this.prefersBorder,
  });

  /// Reads the `app` object of a `tool_result` frame; `null` for any other
  /// shape.
  static McpApp? fromJson(Object? json) {
    if (json is! Map) return null;
    final server = json['server'];
    final resourceUri = json['resource_uri'];
    final html = json['html'];
    if (server is! String || resourceUri is! String || html is! String) {
      return null;
    }
    final csp = json['csp'];
    final permissions = json['permissions'];
    return McpApp(
      server: server,
      resourceUri: resourceUri,
      html: html,
      csp: csp is Map ? McpAppCsp._fromJson(csp) : const McpAppCsp(),
      permissions: permissions is Map
          ? McpAppPermissions._fromJson(permissions)
          : const McpAppPermissions(),
      prefersBorder: switch (json['prefers_border']) {
        final bool value => value,
        _ => null,
      },
    );
  }

  /// Namespace of the server the tool came from, as in
  /// `mcp__{server}__{tool}`.
  final String server;

  /// The `ui://` resource the HTML was read from.
  final String resourceUri;

  final String html;
  final McpAppCsp csp;
  final McpAppPermissions permissions;
  final bool? prefersBorder;
}

/// The origins an app may reach. Empty lists mean the restrictive default.
class McpAppCsp {
  const McpAppCsp({
    this.connectDomains = const [],
    this.resourceDomains = const [],
    this.frameDomains = const [],
    this.baseUriDomains = const [],
  });

  factory McpAppCsp._fromJson(Map<Object?, Object?> json) => McpAppCsp(
    connectDomains: _strings(json['connect_domains']),
    resourceDomains: _strings(json['resource_domains']),
    frameDomains: _strings(json['frame_domains']),
    baseUriDomains: _strings(json['base_uri_domains']),
  );

  final List<String> connectDomains;
  final List<String> resourceDomains;
  final List<String> frameDomains;
  final List<String> baseUriDomains;
}

/// The device permissions an app asks for.
class McpAppPermissions {
  const McpAppPermissions({
    this.camera = false,
    this.microphone = false,
    this.geolocation = false,
    this.clipboardWrite = false,
  });

  factory McpAppPermissions._fromJson(Map<Object?, Object?> json) =>
      McpAppPermissions(
        camera: json['camera'] == true,
        microphone: json['microphone'] == true,
        geolocation: json['geolocation'] == true,
        clipboardWrite: json['clipboard_write'] == true,
      );

  final bool camera;
  final bool microphone;
  final bool geolocation;
  final bool clipboardWrite;
}

List<String> _strings(Object? value) =>
    value is List ? value.whereType<String>().toList() : const [];
