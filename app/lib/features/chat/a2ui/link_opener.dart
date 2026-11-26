import 'dart:async';

/// Opens the target of an A2UI `openUrl` call.
///
/// The catalog calls it only from a user gesture. PR 14b supplies the
/// policy-checked, confirming implementation; until then the default does
/// nothing, so no `url_launcher` call can happen without the seam.
typedef LinkOpener = FutureOr<void> Function(Uri url);

/// The default [LinkOpener]: ignores the link.
void ignoreLink(Uri url) {}
