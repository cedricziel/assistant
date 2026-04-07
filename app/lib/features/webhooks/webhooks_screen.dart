import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'webhooks_provider.dart';

/// Screen that lists all registered webhooks.
class WebhooksScreen extends ConsumerWidget {
  const WebhooksScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final webhooksAsync = ref.watch(webhooksProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Webhooks'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => ref.read(webhooksProvider.notifier).refresh(),
          ),
        ],
      ),
      body: webhooksAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () => ref.read(webhooksProvider.notifier).refresh(),
        ),
        data: (state) {
          if (state.error != null) {
            return _ErrorView(
              error: state.error!,
              onRetry: () => ref.read(webhooksProvider.notifier).refresh(),
            );
          }
          if (state.webhooks.isEmpty) {
            return const _EmptyView();
          }
          return ListView.separated(
            padding: const EdgeInsets.symmetric(vertical: 8),
            itemCount: state.webhooks.length,
            separatorBuilder: (context, index) =>
                const Divider(height: 1, indent: 72),
            itemBuilder: (context, index) {
              return _WebhookRow(webhook: state.webhooks[index]);
            },
          );
        },
      ),
    );
  }
}

class _WebhookRow extends StatelessWidget {
  const _WebhookRow({required this.webhook});

  final WebhookResponse webhook;

  @override
  Widget build(BuildContext context) {
    final isVerified = webhook.verifiedAt != null;

    return ListTile(
      leading: CircleAvatar(
        backgroundColor: webhook.active
            ? Colors.green.shade100
            : Colors.grey.shade200,
        child: Icon(
          Icons.webhook,
          size: 20,
          color: webhook.active
              ? Colors.green.shade700
              : Colors.grey.shade500,
        ),
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
                color: Colors.blue.shade600,
              ),
            ),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
            decoration: BoxDecoration(
              color: webhook.active
                  ? Colors.green.shade50
                  : Colors.grey.shade100,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(
                color: webhook.active
                    ? Colors.green.shade300
                    : Colors.grey.shade400,
              ),
            ),
            child: Text(
              webhook.active ? 'Active' : 'Inactive',
              style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w600,
                color: webhook.active
                    ? Colors.green.shade700
                    : Colors.grey.shade600,
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
    return const Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.webhook, size: 64, color: Colors.black26),
          SizedBox(height: 12),
          Text(
            'No webhooks configured',
            style: TextStyle(fontSize: 16, color: Colors.black45),
          ),
          SizedBox(height: 4),
          Text(
            'Webhooks receive event notifications from the assistant.',
            textAlign: TextAlign.center,
            style: TextStyle(color: Colors.black38),
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
          Icon(Icons.error_outline, color: Colors.red.shade600, size: 48),
          const SizedBox(height: 12),
          Text(error, textAlign: TextAlign.center),
          const SizedBox(height: 12),
          FilledButton(onPressed: onRetry, child: const Text('Retry')),
        ],
      ),
    );
  }
}
