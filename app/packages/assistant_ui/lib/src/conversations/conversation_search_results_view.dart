import 'package:material_ui/material_ui.dart';

import '../models/conversation_search_item.dart';
import '../theme/assistant_ui_spacing.dart';
import 'highlighted_segments.dart';

/// Most snippets shown under one result.
const _maxSnippets = 3;

/// Conversations that matched a search, one row each.
///
/// A row shows the title and up to three message snippets, matched text
/// highlighted, and an "Archived" badge when `archivedAt` is set. Row `i` with
/// id `x` carries the key `conversation_search_result_x`. A tap calls
/// [onSelect]; the view never navigates. An empty [results] shows
/// [emptyLabel].
class ConversationSearchResultsView extends StatelessWidget {
  const ConversationSearchResultsView({
    super.key,
    required this.results,
    required this.onSelect,
    this.emptyLabel = 'No conversations match',
  });

  final List<ConversationSearchItem> results;
  final ValueChanged<String> onSelect;
  final String emptyLabel;

  @override
  Widget build(BuildContext context) {
    if (results.isEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(AssistantUiSpacing.lg),
          child: Text(
            emptyLabel,
            style: TextStyle(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      );
    }

    return ListView.builder(
      itemCount: results.length,
      itemBuilder: (context, i) =>
          _ResultRow(item: results[i], onTap: () => onSelect(results[i].id)),
    );
  }
}

class _ResultRow extends StatelessWidget {
  const _ResultRow({required this.item, required this.onTap});

  final ConversationSearchItem item;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final titleSegments = item.titleSegments.isEmpty
        ? [SearchSegmentItem(text: item.title, highlighted: false)]
        : item.titleSegments;

    return Semantics(
      container: true,
      button: true,
      child: InkWell(
        key: Key('conversation_search_result_${item.id}'),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(
            horizontal: AssistantUiSpacing.lg,
            vertical: AssistantUiSpacing.sm,
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Expanded(
                    child: HighlightedSegments(
                      titleSegments,
                      style: theme.textTheme.titleSmall,
                      maxLines: 1,
                    ),
                  ),
                  if (item.archivedAt != null) ...[
                    const SizedBox(width: AssistantUiSpacing.sm),
                    Text(
                      'Archived',
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ],
              ),
              for (final snippet in item.matches.take(_maxSnippets))
                Padding(
                  padding: const EdgeInsets.only(top: AssistantUiSpacing.xs),
                  child: HighlightedSegments(
                    snippet,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                    maxLines: 2,
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}
