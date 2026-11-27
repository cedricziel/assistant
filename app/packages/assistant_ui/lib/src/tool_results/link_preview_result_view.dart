import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';
import '../scopes/chat_markdown_scope.dart';
import '../theme/assistant_ui_spacing.dart';
import 'tool_result_renderer.dart';

/// Longest title the contract allows, in characters.
const _maxTitleChars = 200;

/// Longest excerpt the contract allows, in characters.
const _maxExcerptChars = 280;

/// The `link_preview` contract: a preview of a fetched web page. See
/// `schemas/tool-results/link_preview.schema.json`.
class LinkPreviewData {
  const LinkPreviewData({
    required this.url,
    required this.finalUrl,
    required this.status,
    this.title,
    this.contentType,
    this.excerpt,
  });

  /// The URL that was fetched.
  final String url;

  /// The URL after redirects; equal to [url] when there was none.
  final String finalUrl;

  /// The HTTP status of the response.
  final int status;

  final String? title;

  /// The response content type, such as `text/html; charset=utf-8`.
  final String? contentType;

  final String? excerpt;

  /// Whether the fetch followed a redirect.
  bool get redirected => finalUrl != url;

  /// The host of [finalUrl], or the whole URL when it does not parse.
  String get finalHost {
    final host = Uri.tryParse(finalUrl)?.host;
    return host == null || host.isEmpty ? finalUrl : host;
  }

  /// [title] when it has text, otherwise the final URL.
  String get heading => switch (title) {
    final String title when title.trim().isNotEmpty => title,
    _ => finalUrl,
  };
}

/// Resolves `kind: "link_preview"` payloads to [LinkPreviewResultView].
class LinkPreviewRenderer extends ToolResultRenderer<LinkPreviewData> {
  const LinkPreviewRenderer();

  @override
  String get kind => 'link_preview';

  @override
  LinkPreviewData? parse(Map<String, Object?> data) {
    final url = data['url'];
    final finalUrl = data['final_url'];
    final status = data['status'];
    final title = data['title'];
    final contentType = data['content_type'];
    final excerpt = data['excerpt'];
    if (data['kind'] != kind ||
        url is! String ||
        finalUrl is! String ||
        status is! int ||
        (title != null &&
            (title is! String || title.runes.length > _maxTitleChars)) ||
        (contentType != null && contentType is! String) ||
        (excerpt != null &&
            (excerpt is! String || excerpt.runes.length > _maxExcerptChars))) {
      return null;
    }
    return LinkPreviewData(
      url: url,
      finalUrl: finalUrl,
      status: status,
      title: title as String?,
      contentType: contentType as String?,
      excerpt: excerpt as String?,
    );
  }

  @override
  Widget summary(BuildContext context, LinkPreviewData model) => Text(
    '${model.heading} · HTTP ${model.status}',
    maxLines: 1,
    overflow: TextOverflow.ellipsis,
    style: Theme.of(context).textTheme.bodySmall,
  );

  @override
  Widget build(
    BuildContext context,
    LinkPreviewData model,
    ToolCallStatus status,
  ) => LinkPreviewResultView(data: model);
}

/// A fetched page as a card: title (or URL), final host, HTTP status badge,
/// content type and excerpt, with a note when the fetch was redirected.
///
/// Activating the card opens [LinkPreviewData.finalUrl] through the nearest
/// [ChatMarkdownScope]; the package performs no navigation itself.
class LinkPreviewResultView extends StatelessWidget {
  const LinkPreviewResultView({super.key, required this.data});

  final LinkPreviewData data;

  void _open(BuildContext context) =>
      ChatMarkdownScope.of(context)?.onTapLink?.call(data.finalUrl);

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final muted = theme.colorScheme.onSurfaceVariant;
    return Semantics(
      container: true,
      button: true,
      label: 'Open ${data.heading}',
      onTap: () => _open(context),
      excludeSemantics: true,
      child: InkWell(
        onTap: () => _open(context),
        borderRadius: BorderRadius.circular(AssistantUiSpacing.sm),
        child: Padding(
          padding: const EdgeInsets.all(AssistantUiSpacing.sm),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(Icons.link, size: 16, color: muted),
                  const SizedBox(width: AssistantUiSpacing.xs),
                  Expanded(
                    child: Text(
                      data.heading,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.titleSmall,
                    ),
                  ),
                  const SizedBox(width: AssistantUiSpacing.xs),
                  _StatusBadge(status: data.status),
                ],
              ),
              const SizedBox(height: AssistantUiSpacing.xs),
              Text(
                data.finalHost,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(color: muted),
              ),
              if (data.redirected) ...[
                const SizedBox(height: AssistantUiSpacing.xs),
                Row(
                  children: [
                    Icon(Icons.arrow_forward, size: 14, color: muted),
                    const SizedBox(width: AssistantUiSpacing.xs),
                    Expanded(
                      child: Text(
                        'Redirected to ${data.finalUrl}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: muted,
                        ),
                      ),
                    ),
                  ],
                ),
              ],
              if (data.contentType case final contentType?) ...[
                const SizedBox(height: AssistantUiSpacing.xs),
                Text(
                  contentType,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.labelSmall?.copyWith(color: muted),
                ),
              ],
              if (data.excerpt case final excerpt?) ...[
                const SizedBox(height: AssistantUiSpacing.sm),
                Text(
                  excerpt,
                  maxLines: 3,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall,
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _StatusBadge extends StatelessWidget {
  const _StatusBadge({required this.status});

  final int status;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final ok = status >= 200 && status < 300;
    final background = ok ? scheme.tertiaryContainer : scheme.errorContainer;
    final foreground = ok
        ? scheme.onTertiaryContainer
        : scheme.onErrorContainer;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        'HTTP $status',
        style: Theme.of(context).textTheme.labelSmall
            ?.copyWith(color: foreground, fontWeight: FontWeight.w600),
      ),
    );
  }
}
