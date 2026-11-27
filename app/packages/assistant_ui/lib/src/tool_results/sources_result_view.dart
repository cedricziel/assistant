import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';
import '../scopes/chat_markdown_scope.dart';
import '../theme/assistant_ui_spacing.dart';
import 'tool_result_renderer.dart';

/// One row of a `sources` result.
class SourceResult {
  const SourceResult({
    required this.title,
    required this.url,
    required this.domain,
    this.snippet,
  });

  final String title;
  final String url;
  final String domain;
  final String? snippet;
}

/// The `sources` contract: the results of a web search. See
/// `schemas/tool-results/sources.schema.json`.
class SourcesData {
  const SourcesData({required this.query, required this.results});

  final String query;
  final List<SourceResult> results;
}

/// Resolves `kind: "sources"` payloads to [SourcesResultView].
class SourcesRenderer extends ToolResultRenderer<SourcesData> {
  const SourcesRenderer();

  @override
  String get kind => 'sources';

  @override
  SourcesData? parse(Map<String, Object?> data) {
    final query = data['query'];
    final rawResults = data['results'];
    if (data['kind'] != kind || query is! String || rawResults is! List) {
      return null;
    }
    final results = <SourceResult>[];
    for (final raw in rawResults) {
      if (raw is! Map) return null;
      final title = raw['title'];
      final url = raw['url'];
      final domain = raw['domain'];
      final snippet = raw['snippet'];
      if (title is! String ||
          url is! String ||
          domain is! String ||
          (snippet != null && snippet is! String)) {
        return null;
      }
      results.add(
        SourceResult(
          title: title,
          url: url,
          domain: domain,
          snippet: snippet as String?,
        ),
      );
    }
    return SourcesData(query: query, results: results);
  }

  @override
  Widget summary(BuildContext context, SourcesData model) {
    final count = model.results.length;
    final text = switch (count) {
      0 => 'No sources',
      1 => '1 source',
      _ => '$count sources',
    };
    return Text(
      text,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: Theme.of(context).textTheme.bodySmall,
    );
  }

  @override
  Widget build(
    BuildContext context,
    SourcesData model,
    ToolCallStatus status,
  ) => SourcesResultView(data: model);
}

/// A web search's query and one tappable row per result.
///
/// A row leaves the package only through [ChatMarkdownScope.onTapLink]; the
/// view itself never opens a URL and fetches nothing.
class SourcesResultView extends StatelessWidget {
  const SourcesResultView({super.key, required this.data});

  final SourcesData data;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Icon(Icons.search, size: 16, color: scheme.onSurfaceVariant),
            const SizedBox(width: AssistantUiSpacing.xs),
            Expanded(
              child: Text(
                data.query,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.labelLarge,
              ),
            ),
          ],
        ),
        if (data.results.isEmpty)
          Padding(
            padding: const EdgeInsets.only(top: AssistantUiSpacing.xs),
            child: Text(
              'No results',
              style: theme.textTheme.bodySmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
          )
        else
          for (final result in data.results) _SourceRow(result: result),
      ],
    );
  }
}

class _SourceRow extends StatelessWidget {
  const _SourceRow({required this.result});

  final SourceResult result;

  void _open(BuildContext context) =>
      ChatMarkdownScope.of(context)?.onTapLink?.call(result.url);

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final snippet = result.snippet;
    return InkWell(
      onTap: () => _open(context),
      borderRadius: BorderRadius.circular(8),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: AssistantUiSpacing.xs),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const SizedBox(height: AssistantUiSpacing.xs),
            Text(
              result.title,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: scheme.primary,
              ),
            ),
            const SizedBox(height: 2),
            Text(
              result.domain,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.labelSmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
            if (snippet != null && snippet.isNotEmpty) ...[
              const SizedBox(height: 2),
              Text(
                snippet,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall,
              ),
            ],
          ],
        ),
      ),
    );
  }
}
