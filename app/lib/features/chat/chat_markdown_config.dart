import 'package:assistant_ui/assistant_ui.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart'
    hide ToolCallStatus;
import 'package:flutter_svg/flutter_svg.dart';
import 'package:material_ui/material_ui.dart';

import 'markdown_link_handler.dart';
import 'svg_builder.dart';
import 'theme_aware_mermaid.dart';

/// Installs the app's [ChatMarkdownScope]: the Mermaid and SVG plugins and
/// builders, link launching through [MarkdownLinkHandler], network images and
/// the code-copy callback supplied by the caller.
///
/// The assistant_ui package does no I/O, so everything that does lives here.
/// The registries are rebuilt only when the colour scheme changes, so scope
/// consumers do not rebuild on every parent rebuild.
class ChatMarkdownConfig extends StatefulWidget {
  const ChatMarkdownConfig({super.key, this.onCopyCode, required this.child});

  /// Receives the code of a copied code block; `null` hides the copy button.
  final ValueChanged<String>? onCopyCode;

  final Widget child;

  @override
  State<ChatMarkdownConfig> createState() => _ChatMarkdownConfigState();
}

class _ChatMarkdownConfigState extends State<ChatMarkdownConfig> {
  static final ParserPluginRegistry _plugins = ParserPluginRegistry()
    ..registerBlock(const MermaidPlugin())
    ..registerBlock(const SvgPlugin());

  ColorScheme? _colorScheme;
  BuilderRegistry? _builders;

  BuilderRegistry _buildersFor(ColorScheme colorScheme) {
    if (_builders == null || _colorScheme != colorScheme) {
      _colorScheme = colorScheme;
      _builders = BuilderRegistry()
        ..register(
          'mermaid',
          ThemeAwareMermaidBuilder(
            style: mermaidStyleFromColorScheme(colorScheme),
          ),
        )
        ..register('svg', const SvgBuilder());
    }
    return _builders!;
  }

  void _onTapLink(String url) =>
      MarkdownLinkHandler(context: context).onTap(url);

  @override
  Widget build(BuildContext context) {
    return ChatMarkdownScope(
      plugins: _plugins,
      builders: _buildersFor(Theme.of(context).colorScheme),
      imageBuilder: networkMarkdownImage,
      onTapLink: _onTapLink,
      onCopyCode: widget.onCopyCode,
      child: widget.child,
    );
  }
}

/// Renders a markdown image over the network as `flutter_smooth_markdown`'s
/// default image builder does: `SvgPicture` for `.svg`, `CachedNetworkImage`
/// for other `http(s)` URLs, and an asset image otherwise.
Widget networkMarkdownImage(String url, String? alt, String? title) {
  final isNetwork = url.startsWith('http://') || url.startsWith('https://');
  const loading = Center(child: CircularProgressIndicator());

  if (url.toLowerCase().endsWith('.svg')) {
    return isNetwork
        ? SvgPicture.network(url, placeholderBuilder: (_) => loading)
        : SvgPicture.asset(url, placeholderBuilder: (_) => loading);
  }
  if (isNetwork) {
    return CachedNetworkImage(
      imageUrl: url,
      placeholder: (_, _) => loading,
      errorWidget: (_, _, _) => const Icon(Icons.error),
    );
  }
  return Image.asset(
    url,
    errorBuilder: (_, _, _) => const Icon(Icons.broken_image),
  );
}
