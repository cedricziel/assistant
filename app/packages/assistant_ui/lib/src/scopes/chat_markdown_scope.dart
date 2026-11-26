import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart'
    hide ToolCallStatus;
import 'package:material_ui/material_ui.dart';

/// Builds the widget for a markdown image, as `flutter_smooth_markdown`'s
/// `imageBuilder` parameter expects.
typedef ChatMarkdownImageBuilder = Widget Function(
  String url,
  String? alt,
  String? title,
);

/// Supplies the markdown configuration of a thread to every body below it:
/// parser [plugins], widget [builders], the [imageBuilder], link taps and a
/// style sheet override.
///
/// The package does no I/O, so the app registers whatever does: a launcher for
/// [onTapLink], network images in [imageBuilder], and plugins such as Mermaid
/// or SVG. Without an ancestor scope, [of] returns `null`: no plugins, links
/// ignored, and images render [markdownImagePlaceholder] (see [imageBuilderOf]).
///
/// A body that renders markdown reads the pieces it applies. `ThinkingBody`
/// applies only [onTapLink] and [imageBuilder]: reasoning has no diagrams.
class ChatMarkdownScope extends InheritedWidget {
  const ChatMarkdownScope({
    super.key,
    this.plugins,
    this.builders,
    this.imageBuilder,
    this.onTapLink,
    this.styleSheetOverride,
    required super.child,
  });

  /// Parser plugins for extra block or inline syntax.
  final ParserPluginRegistry? plugins;

  /// Widget builders for the nodes [plugins] produce, and for replacing
  /// built-in ones such as code blocks.
  final BuilderRegistry? builders;

  /// Renders a markdown image. `null` means [markdownImagePlaceholder].
  final ChatMarkdownImageBuilder? imageBuilder;

  /// Called with the URL of a tapped link. `null` ignores link taps.
  final void Function(String url)? onTapLink;

  /// Replaces the theme-derived style sheet of message markdown. `null` keeps
  /// the theme's.
  final MarkdownStyleSheet? styleSheetOverride;

  /// The nearest scope, or `null` when there is none. Rebuilds the caller when
  /// the scope changes.
  static ChatMarkdownScope? of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<ChatMarkdownScope>();

  /// The image builder every markdown widget in the package must pass: the
  /// scope's, or the placeholder, so `flutter_smooth_markdown` never reaches
  /// its default network image loading.
  static ChatMarkdownImageBuilder imageBuilderOf(BuildContext context) =>
      of(context)?.imageBuilder ?? markdownImagePlaceholder;

  @override
  bool updateShouldNotify(ChatMarkdownScope oldWidget) =>
      plugins != oldWidget.plugins ||
      builders != oldWidget.builders ||
      imageBuilder != oldWidget.imageBuilder ||
      onTapLink != oldWidget.onTapLink ||
      styleSheetOverride != oldWidget.styleSheetOverride;
}

/// The no-I/O stand-in for a markdown image: its alt text (or title, or URL)
/// with an image icon. Used when no [ChatMarkdownScope] supplies an image
/// builder. The markdown renderer adds the image semantics.
Widget markdownImagePlaceholder(String url, String? alt, String? title) {
  final label = [alt, title, url].firstWhere((s) => s != null && s.isNotEmpty)!;
  return Builder(
    builder: (context) {
      final color = Theme.of(context).colorScheme.onSurfaceVariant;
      return Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.image_outlined, size: 16, color: color),
          const SizedBox(width: 4),
          Flexible(
            child: Text(label, style: TextStyle(color: color)),
          ),
        ],
      );
    },
  );
}
