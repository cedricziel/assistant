import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../shared/platform/adaptive_dialog.dart';
import '../markdown_link_handler.dart' show UrlLauncher;
import 'url_policy.dart';

/// Opens the target of an A2UI `openUrl` call.
///
/// The catalog calls it only from a user gesture. [confirmingLinkOpener]
/// supplies the policy-checked implementation; [ignoreLink] is the default
/// when a catalog is built without one.
typedef LinkOpener = FutureOr<void> Function(Uri url);

/// The default [LinkOpener]: ignores the link.
void ignoreLink(Uri url) {}

Future<bool> _launch(Uri uri, {LaunchMode mode = LaunchMode.platformDefault}) =>
    launchUrl(uri, mode: mode);

/// The [LinkOpener] the app installs in `assistantCatalog()`.
///
/// A target that violates [UrlPolicy.link] is dropped: it never reaches the
/// dialog or the launcher. An `https:` host that equals an entry of
/// [linkHosts] (case-insensitive, exact) launches directly; every other
/// `https:` target and every `mailto:` target asks first, showing the full
/// destination with the host emphasised. Cancel launches nothing.
///
/// [context] supplies the context the dialog is shown on, read when the user
/// taps. [linkHosts] is read at the same moment, so a late capabilities
/// response applies to the next tap.
LinkOpener confirmingLinkOpener({
  required BuildContext Function() context,
  required List<String> Function() linkHosts,
  UrlLauncher launcher = _launch,
}) => (url) async {
  final target = UrlPolicy.link(url.toString());
  if (target == null) return;

  if (!_isTrusted(target, linkHosts())) {
    final dialogContext = context();
    if (!dialogContext.mounted) return;
    final confirmed = await showAdaptiveConfirmDialog(
      context: dialogContext,
      title: 'Open link?',
      content: target.toString(),
      body: SingleChildScrollView(child: _Destination(target)),
      confirmLabel: 'Open',
    );
    if (!confirmed) return;
  }

  try {
    await launcher(target, mode: LaunchMode.externalApplication);
  } catch (e, st) {
    debugPrint('A2UI link launch failed: $e\n$st');
  }
};

bool _isTrusted(Uri target, List<String> linkHosts) =>
    target.scheme == 'https' &&
    linkHosts.any((h) => h.toLowerCase() == target.host.toLowerCase());

/// The full destination, with the host in bold so a long path or a
/// look-alike prefix cannot hide where the link goes.
class _Destination extends StatelessWidget {
  const _Destination(this.target);

  final Uri target;

  @override
  Widget build(BuildContext context) {
    final text = target.toString();
    final authorityStart = text.indexOf('//') + 2;
    // The user info comes first; a host repeated in it must not be the one
    // emphasised.
    final hostStart = target.host.isEmpty
        ? -1
        : text.indexOf(
            target.host,
            authorityStart +
                (target.userInfo.isEmpty ? 0 : target.userInfo.length + 1),
          );
    final style = DefaultTextStyle.of(context).style;
    if (hostStart < 0) {
      // mailto: has no host; the address is the destination.
      return Text(text, style: style.copyWith(fontWeight: FontWeight.bold));
    }
    final hostEnd = hostStart + target.host.length;
    return Text.rich(
      TextSpan(
        style: style,
        children: [
          TextSpan(text: text.substring(0, hostStart)),
          TextSpan(
            text: target.host,
            style: const TextStyle(fontWeight: FontWeight.bold),
          ),
          TextSpan(text: text.substring(hostEnd)),
        ],
      ),
    );
  }
}
