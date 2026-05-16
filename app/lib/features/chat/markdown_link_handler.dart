import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

typedef UrlLauncher = Future<bool> Function(Uri uri, {LaunchMode mode});

Future<bool> _defaultLauncher(
  Uri uri, {
  LaunchMode mode = LaunchMode.platformDefault,
}) {
  return launchUrl(uri, mode: mode);
}

const Set<String> _allowedSchemes = {'http', 'https', 'mailto'};

class MarkdownLinkHandler {
  MarkdownLinkHandler({
    required this.context,
    UrlLauncher launcher = _defaultLauncher,
  }) : _launcher = launcher;

  final BuildContext context;
  final UrlLauncher _launcher;

  Future<void> onTap(String url) async {
    final uri = Uri.tryParse(url);
    if (uri == null ||
        uri.scheme.isEmpty ||
        !_allowedSchemes.contains(uri.scheme)) {
      _showSnackBar('Cannot open link: $url');
      return;
    }

    try {
      final ok = await _launcher(uri, mode: LaunchMode.externalApplication);
      if (!ok) {
        _showSnackBar('Could not open link: $url');
      }
    } catch (e, st) {
      debugPrint('MarkdownLinkHandler launch failed: $e\n$st');
      _showSnackBar('Could not open link');
    }
  }

  void _showSnackBar(String message) {
    final messenger = ScaffoldMessenger.maybeOf(context);
    if (messenger == null) return;
    messenger.showSnackBar(SnackBar(content: Text(message)));
  }
}
