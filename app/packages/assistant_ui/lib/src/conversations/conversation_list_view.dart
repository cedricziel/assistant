import 'package:material_ui/material_ui.dart';

/// A conversation as the list shows it.
class ConversationListItem {
  const ConversationListItem({
    required this.id,
    required this.title,
    required this.updatedAt,
  });

  final String id;
  final String title;
  final DateTime updatedAt;
}

/// Builds the optional trailing widget of one row.
typedef ConversationTrailingBuilder = Widget? Function(
  BuildContext context,
  ConversationListItem item,
);

/// A scrolling list of conversations with tap-to-open and swipe-to-delete.
///
/// The view owns no state and never navigates: a tap calls [onSelect] with the
/// conversation id. A swipe calls [onDelete], which the caller implements as
/// confirm-then-delete; the row is dismissed only when it resolves to `true`,
/// so a declined confirmation or a failed delete leaves the row in place.
///
/// [header] sits above the rows and stays visible for an empty list, where
/// [emptyState] fills the remaining space. Relative dates are computed against
/// [now], which defaults to the current time. No row is highlighted as
/// selected.
class ConversationListView extends StatelessWidget {
  const ConversationListView({
    super.key,
    required this.items,
    required this.onSelect,
    required this.onDelete,
    this.header,
    this.emptyState,
    this.trailingBuilder,
    this.now,
  });

  final List<ConversationListItem> items;
  final ValueChanged<String> onSelect;
  final Future<bool> Function(String id) onDelete;
  final Widget? header;
  final Widget? emptyState;
  final ConversationTrailingBuilder? trailingBuilder;
  final DateTime? now;

  @override
  Widget build(BuildContext context) {
    final reference = now ?? DateTime.now();
    final scheme = Theme.of(context).colorScheme;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ?header,
        Expanded(
          child: items.isEmpty
              ? emptyState ?? const SizedBox.shrink()
              : ListView.builder(
                  itemCount: items.length,
                  itemBuilder: (context, index) {
                    final item = items[index];
                    return Dismissible(
                      key: ValueKey(item.id),
                      direction: DismissDirection.endToStart,
                      background: Container(
                        alignment: Alignment.centerRight,
                        padding: const EdgeInsets.only(right: 16),
                        color: scheme.error,
                        child: Icon(Icons.delete, color: scheme.onError),
                      ),
                      confirmDismiss: (_) => onDelete(item.id),
                      child: ListTile(
                        title: Text(
                          item.title,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        subtitle: Text(
                          _formatDate(reference, item.updatedAt),
                          style: const TextStyle(fontSize: 11),
                        ),
                        trailing: trailingBuilder?.call(context, item),
                        onTap: () => onSelect(item.id),
                      ),
                    );
                  },
                ),
        ),
      ],
    );
  }

  static String _formatDate(DateTime now, DateTime then) {
    final diff = now.difference(then);
    if (diff.inMinutes < 1) return 'Just now';
    if (diff.inHours < 1) return '${diff.inMinutes}m ago';
    if (diff.inDays < 1) return '${diff.inHours}h ago';
    return '${diff.inDays}d ago';
  }
}
