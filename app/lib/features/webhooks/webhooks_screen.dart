import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../shared/platform/widgets.dart';
import 'webhooks_provider.dart';

/// Screen that lists all registered webhooks.
class WebhooksScreen extends ConsumerWidget {
  const WebhooksScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final webhooksAsync = ref.watch(webhooksProvider);

    return AdaptiveScaffold(
      body: CustomScrollView(
        slivers: [
          AdaptiveSliverNavBar(
            title: 'Webhooks',
            actions: [
              IconButton(
                icon: const Icon(Icons.refresh),
                onPressed: () => ref.read(webhooksProvider.notifier).refresh(),
              ),
            ],
          ),
          webhooksAsync.when(
            loading: () => const SliverFillRemaining(
              child: Center(child: CircularProgressIndicator.adaptive()),
            ),
            error: (err, _) => SliverFillRemaining(
              child: _ErrorView(
                error: err.toString(),
                onRetry: () => ref.read(webhooksProvider.notifier).refresh(),
              ),
            ),
            data: (state) {
              if (state.error != null) {
                return SliverFillRemaining(
                  child: _ErrorView(
                    error: state.error!,
                    onRetry: () =>
                        ref.read(webhooksProvider.notifier).refresh(),
                  ),
                );
              }
              if (state.webhooks.isEmpty) {
                return const SliverFillRemaining(child: _EmptyView());
              }
              return SliverList(
                delegate: SliverChildBuilderDelegate((context, index) {
                  if (index.isOdd) {
                    return const Divider(height: 1, indent: 72);
                  }
                  return _WebhookRow(webhook: state.webhooks[index ~/ 2]);
                }, childCount: state.webhooks.length * 2 - 1),
              );
            },
          ),
        ],
      ),
    );
  }
}

class _WebhookRow extends StatelessWidget {
  const _WebhookRow({required this.webhook});

  final WebhookResponse webhook;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isVerified = webhook.verifiedAt != null;
    // Active uses tertiary (success token); inactive uses dim outline.
    final swatch = webhook.active
        ? colorScheme.tertiary
        : colorScheme.onSurfaceVariant;
    final softFill = webhook.active
        ? colorScheme.tertiaryContainer
        : colorScheme.surfaceContainerHighest;

    return AdaptiveListTile(
      leading: CircleAvatar(
        backgroundColor: softFill,
        child: Icon(Icons.webhook, size: 20, color: swatch),
      ),
      title: Text(
        webhook.name,
        style: const TextStyle(fontWeight: FontWeight.w600),
      ),
      subtitle: Text(
        webhook.url,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: const TextStyle(fontSize: 12),
      ),
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (isVerified)
            Padding(
              padding: const EdgeInsets.only(right: 4),
              child: Icon(
                Icons.verified_outlined,
                size: 16,
                color: colorScheme.primary,
              ),
            ),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
            decoration: BoxDecoration(
              color: softFill,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: swatch.withValues(alpha: 0.4)),
            ),
            child: Text(
              webhook.active ? 'Active' : 'Inactive',
              style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w600,
                color: swatch,
              ),
            ),
          ),
        ],
      ),
      onTap: () => context.go('/webhooks/${webhook.id}'),
    );
  }
}

class _EmptyView extends StatelessWidget {
  const _EmptyView();

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.webhook, size: 64, color: colorScheme.outlineVariant),
          const SizedBox(height: 12),
          Text(
            'No webhooks configured',
            style: TextStyle(fontSize: 16, color: colorScheme.onSurfaceVariant),
          ),
          const SizedBox(height: 4),
          Text(
            'Webhooks receive event notifications from the assistant.',
            textAlign: TextAlign.center,
            style: TextStyle(color: colorScheme.onSurfaceVariant),
          ),
        ],
      ),
    );
  }
}

class _ErrorView extends StatelessWidget {
  const _ErrorView({required this.error, required this.onRetry});

  final String error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.error_outline,
            color: Theme.of(context).colorScheme.error,
            size: 48,
          ),
          const SizedBox(height: 12),
          Text(error, textAlign: TextAlign.center),
          const SizedBox(height: 12),
          AdaptiveButton.filled(onPressed: onRetry, child: const Text('Retry')),
        ],
      ),
    );
  }
}
