import 'package:assistant_ui/models.dart';

/// The Content-Security-Policy for an app, built only from its declared
/// domains. With none declared it is the MCP Apps restrictive default.
String mcpAppCsp(McpAppCsp csp) {
  final resources = _sources(csp.resourceDomains);
  final connect = _sources(csp.connectDomains);
  final frames = _sources(csp.frameDomains);
  final baseUris = _sources(csp.baseUriDomains);
  String directive(String name, List<String> sources) =>
      [name, ...sources].join(' ');

  return [
    "default-src 'none'",
    directive('script-src', ["'self'", "'unsafe-inline'", ...resources]),
    directive('style-src', ["'self'", "'unsafe-inline'", ...resources]),
    directive('img-src', ["'self'", 'data:', ...resources]),
    if (resources.isNotEmpty) directive('font-src', ["'self'", ...resources]),
    directive('media-src', ["'self'", 'data:', ...resources]),
    directive('connect-src', connect.isEmpty ? ["'none'"] : connect),
    directive('frame-src', frames.isEmpty ? ["'none'"] : frames),
    directive('base-uri', baseUris.isEmpty ? ["'self'"] : baseUris),
    "object-src 'none'",
  ].join('; ');
}

/// A declared domain is kept only when it is a plain host source, so it
/// cannot add keywords or directives to the policy.
final _hostSource = RegExp(r'^[A-Za-z0-9*.:/\[\]%_~+-]+$');

List<String> _sources(List<String> domains) =>
    domains.where(_hostSource.hasMatch).toList();

/// The iframe `allow` attribute for an app's device permissions.
String mcpAppAllow(McpAppPermissions permissions) => [
  if (permissions.camera) 'camera',
  if (permissions.microphone) 'microphone',
  if (permissions.geolocation) 'geolocation',
  if (permissions.clipboardWrite) 'clipboard-write',
].join('; ');

final _leadingDoctype = RegExp(r'^\s*<!doctype[^>]*>', caseSensitive: false);

/// The app's HTML with its CSP injected as a `<meta http-equiv>` ahead of any
/// app content, followed by [bootstrapScript] when given.
///
/// The policy goes right after the doctype rather than into `<head>`, so no
/// app markup placed before `<head>` runs outside it.
String mcpAppDocument(McpApp app, {String? bootstrapScript}) {
  final policy = mcpAppCsp(app.csp)
      .replaceAll('&', '&amp;')
      .replaceAll('"', '&quot;');
  final prelude = [
    '<meta http-equiv="Content-Security-Policy" content="$policy">',
    if (bootstrapScript != null) '<script>$bootstrapScript</script>',
  ].join();
  final doctype = _leadingDoctype.matchAsPrefix(app.html)?.group(0) ?? '';
  return '$doctype$prelude${app.html.substring(doctype.length)}';
}
