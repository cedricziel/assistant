import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart'
    hide ToolCallStatus;
import 'package:material_ui/material_ui.dart';

import '../scopes/chat_markdown_scope.dart';
import '../theme/legacy_theme.dart';

/// Builds the toolbar shown over selected markdown text, as
/// `flutter_smooth_markdown`'s `contextMenuBuilder` expects.
typedef MessageSelectionToolbarBuilder = Widget Function(
  BuildContext context,
  SmoothSelectionRegionState selectableRegionState,
);

/// The markdown content of an assistant message.
///
/// While [isStreaming] and a [tokenStream] is supplied, the stream renders
/// through `StreamMarkdown`; otherwise [content] renders through a selectable
/// `SmoothMarkdown`. Empty content renders nothing.
///
/// Plugins, builders, the image builder, link taps and a style sheet override
/// come from the nearest [ChatMarkdownScope]. The package does no I/O, so
/// without a scope images render [markdownImagePlaceholder] and link taps are
/// ignored.
class MessageBody extends StatelessWidget {
  const MessageBody({
    super.key,
    required this.content,
    this.tokenStream,
    this.isStreaming = false,
    this.selectionToolbarBuilder,
  });

  /// The accumulated message text.
  final String content;

  /// Live token chunks; rendered only while [isStreaming].
  final Stream<String>? tokenStream;
  final bool isStreaming;

  /// Replaces the toolbar of selected text in the static rendering. `null`
  /// keeps the platform default.
  final MessageSelectionToolbarBuilder? selectionToolbarBuilder;

  @override
  Widget build(BuildContext context) {
    final scope = ChatMarkdownScope.of(context);
    final colorScheme = Theme.of(context).colorScheme;
    final styleSheet =
        scope?.styleSheetOverride ??
        MarkdownStyleSheet.fromTheme(legacyThemeOf(context)).copyWith(
          paragraphStyle: TextStyle(color: colorScheme.onSurface),
          inlineCodeStyle: TextStyle(
            color: colorScheme.onSurface,
            backgroundColor: colorScheme.surfaceContainerLowest,
          ),
        );
    final imageBuilder = ChatMarkdownScope.imageBuilderOf(context);

    final stream = tokenStream;
    if (isStreaming && stream != null) {
      return StreamMarkdown(
        stream: stream,
        styleSheet: styleSheet,
        useEnhancedComponents: true,
        imageBuilder: imageBuilder,
        onTapLink: scope?.onTapLink,
        plugins: scope?.plugins,
        builderRegistry: scope?.builders,
      );
    }
    if (content.isEmpty) return const SizedBox.shrink();
    return SmoothMarkdown(
      data: content,
      styleSheet: styleSheet,
      selectable: true,
      useEnhancedComponents: true,
      imageBuilder: imageBuilder,
      onTapLink: scope?.onTapLink,
      plugins: scope?.plugins,
      builderRegistry: scope?.builders,
      contextMenuBuilder: selectionToolbarBuilder,
    );
  }
}
