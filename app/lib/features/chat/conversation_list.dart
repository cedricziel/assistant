import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import '../../shared/platform/adaptive_dialog.dart';
import 'chat_provider.dart';
import 'conversation_search_field.dart';
import 'conversation_search_provider.dart';

/// Sidebar/drawer pane that shows the list of conversations.
///
/// Adapter around the pure [ConversationListView]: it reads
/// [conversationListProvider], maps entries to [ConversationListItem]s, and
/// supplies navigation, the "New Chat" button and the delete confirmation.
class ConversationListPane extends ConsumerWidget {
  const ConversationListPane({
    super.key,
    this.onConversationSelected,
    this.selectedId,
  });

  /// Id of the open conversation, highlighted in the list.
  final String? selectedId;

  /// Optional callback invoked when a conversation is tapped (e.g. to close
  /// the drawer on mobile).
  final VoidCallback? onConversationSelected;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final searching = ref.watch(
      conversationSearchQueryProvider.select((q) => q.trim().isNotEmpty),
    );
    // Watched even while idle so the search provider exists before the first
    // keystroke and sees the query change.
    final search = ref.watch(conversationSearchProvider);
    // The header stays outside the swapped body so the search field keeps its
    // state and focus while results replace the list.
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _buildHeader(context, ref),
        Expanded(
          child: searching
              ? _buildSearchBody(context, ref, search)
              : _buildListBody(context, ref),
        ),
      ],
    );
  }

  Widget _buildSearchBody(
    BuildContext context,
    WidgetRef ref,
    ConversationSearchState search,
  ) {
    return switch (search) {
      ConversationSearchIdle() || ConversationSearchLoading() => const Center(
        child: CircularProgressIndicator.adaptive(),
      ),
      ConversationSearchError() => _errorBody(
        context,
        'Search failed',
        ref.read(conversationSearchProvider.notifier).retry,
      ),
      ConversationSearchData(:final results) => ConversationSearchResultsView(
        results: results,
        onSelect: (id) {
          ref.read(conversationSearchQueryProvider.notifier).clear();
          context.go('/chat/$id');
          onConversationSelected?.call();
        },
      ),
    };
  }

  Widget _buildListBody(BuildContext context, WidgetRef ref) {
    final listAsync = ref.watch(conversationListProvider);
    return listAsync.when(
      loading: () => const Center(child: CircularProgressIndicator.adaptive()),
      error: (err, _) => _errorBody(
        context,
        'Failed to load conversations',
        ref.read(conversationListProvider.notifier).refresh,
      ),
      data: (listState) {
        final items = [
          for (final conv in listState.conversations)
            ConversationListItem(
              id: conv.id,
              title: conv.title,
              updatedAt: conv.updatedAt,
              pinnedAt: conv.pinnedAt,
            ),
        ];
        return ConversationListView(
          items: items,
          selectedId: selectedId,
          emptyState: Center(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Text(
                'No conversations yet.\nTap "New Chat" to start.',
                textAlign: TextAlign.center,
                style: TextStyle(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          ),
          onSelect: (id) {
            context.go('/chat/$id');
            onConversationSelected?.call();
          },
          onDelete: (id) {
            final title = items.where((i) => i.id == id).firstOrNull?.title;
            return _confirmAndDelete(context, ref, id, title ?? '');
          },
        );
      },
    );
  }

  Widget _errorBody(BuildContext context, String message, VoidCallback retry) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.error_outline, color: Theme.of(context).colorScheme.error),
          const SizedBox(height: 8),
          Text(
            message,
            style: TextStyle(
              color: Theme.of(context).colorScheme.onErrorContainer,
            ),
          ),
          TextButton(onPressed: retry, child: const Text('Retry')),
        ],
      ),
    );
  }

  Widget _buildHeader(BuildContext context, WidgetRef ref) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
          child: Text(
            'Conversations',
            style: Theme.of(context).textTheme.titleMedium
                ?.copyWith(fontWeight: FontWeight.bold),
          ),
        ),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
          child: OutlinedButton.icon(
            key: const Key('new_chat_button'),
            onPressed: () async {
              final notifier = ref.read(conversationListProvider.notifier);
              final createdId = await notifier.createConversation();
              if (createdId != null) {
                ref.read(chatProvider.notifier).clearConversation();
                if (context.mounted) {
                  context.go('/chat/$createdId');
                  onConversationSelected?.call();
                }
              }
            },
            icon: const Icon(Icons.add, size: 18),
            label: const Text('New Chat'),
          ),
        ),
        const ConversationSearchField(),
        const Divider(height: 8),
      ],
    );
  }

  /// Confirms, then deletes. Resolves `false` on decline or failure so the
  /// row stays in the list.
  Future<bool> _confirmAndDelete(
    BuildContext context,
    WidgetRef ref,
    String id,
    String title,
  ) async {
    final confirm = await showAdaptiveConfirmDialog(
      context: context,
      title: 'Delete conversation?',
      content: '"$title" will be permanently deleted.',
      confirmLabel: 'Delete',
      isDestructive: true,
    );
    if (!confirm) return false;
    try {
      await ref.read(conversationListProvider.notifier).deleteConversation(id);
      return true;
    } catch (_) {
      return false;
    }
  }
}
