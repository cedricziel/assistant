import 'package:flutter/material.dart';

import '../../shared/platform/adaptive_dialog.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'chat_provider.dart';

/// Sidebar/drawer widget that shows the list of conversations.
///
/// - Displays a "New Chat" button at the top.
/// - Lists conversations with tap-to-open and swipe-to-delete.
class ConversationList extends ConsumerWidget {
  const ConversationList({super.key, this.onConversationSelected});

  /// Optional callback invoked when a conversation is tapped (e.g. to close
  /// the drawer on mobile).
  final VoidCallback? onConversationSelected;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final listAsync = ref.watch(conversationListProvider);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Header
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
          child: Text(
            'Conversations',
            style: Theme.of(
              context,
            ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
          ),
        ),

        // New Chat button
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
          child: OutlinedButton.icon(
            key: const Key('new_chat_button'),
            onPressed: () async {
              final notifier = ref.read(conversationListProvider.notifier);
              final createdId = await notifier.createConversation(
                title: 'New Chat',
              );
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

        const Divider(height: 8),

        // Conversation list
        Expanded(
          child: listAsync.when(
            loading: () =>
                const Center(child: CircularProgressIndicator.adaptive()),
            error: (err, _) => Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    Icons.error_outline,
                    color: Theme.of(context).colorScheme.error,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Failed to load conversations',
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.onErrorContainer,
                    ),
                  ),
                  TextButton(
                    onPressed: () =>
                        ref.read(conversationListProvider.notifier).refresh(),
                    child: const Text('Retry'),
                  ),
                ],
              ),
            ),
            data: (listState) {
              if (listState.conversations.isEmpty) {
                return Center(
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
                );
              }

              return ListView.builder(
                itemCount: listState.conversations.length,
                itemBuilder: (context, index) {
                  final conv = listState.conversations[index];
                  return Dismissible(
                    key: ValueKey(conv.id),
                    direction: DismissDirection.endToStart,
                    background: Container(
                      alignment: Alignment.centerRight,
                      padding: const EdgeInsets.only(right: 16),
                      color: Theme.of(context).colorScheme.error,
                      child: Icon(
                        Icons.delete,
                        color: Theme.of(context).colorScheme.onError,
                      ),
                    ),
                    confirmDismiss: (_) async {
                      final confirm = await showAdaptiveConfirmDialog(
                        context: context,
                        title: 'Delete conversation?',
                        content: '"${conv.title}" will be permanently deleted.',
                        confirmLabel: 'Delete',
                        isDestructive: true,
                      );
                      if (!confirm) return false;
                      try {
                        await ref
                            .read(conversationListProvider.notifier)
                            .deleteConversation(conv.id);
                        return true;
                      } catch (_) {
                        return false;
                      }
                    },
                    child: ListTile(
                      title: Text(
                        conv.title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      subtitle: Text(
                        _formatDate(conv.updatedAt),
                        style: const TextStyle(fontSize: 11),
                      ),
                      onTap: () {
                        context.go('/chat/${conv.id}');
                        onConversationSelected?.call();
                      },
                    ),
                  );
                },
              );
            },
          ),
        ),
      ],
    );
  }

  String _formatDate(DateTime dt) {
    final now = DateTime.now();
    final diff = now.difference(dt);
    if (diff.inMinutes < 1) return 'Just now';
    if (diff.inHours < 1) return '${diff.inMinutes}m ago';
    if (diff.inDays < 1) return '${diff.inHours}h ago';
    return '${diff.inDays}d ago';
  }
}
