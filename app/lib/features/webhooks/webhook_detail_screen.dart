import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_api/assistant_api.dart';

import 'webhooks_provider.dart';

/// Screen that shows the detail of a single webhook.
class WebhookDetailScreen extends ConsumerWidget {
  const WebhookDetailScreen({super.key, required this.webhookId});

  final String webhookId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final detailAsync = ref.watch(webhookDetailProvider(webhookId));

    return Scaffold(
      appBar: AppBar(
        title: Text(
          detailAsync.value?.webhook?.name ?? 'Webhook',
        ),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/webhooks'),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () =>
                ref.read(webhookDetailProvider(webhookId).notifier).refresh(),
          ),
        ],
      ),
      body: detailAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () =>
              ref.read(webhookDetailProvider(webhookId).notifier).refresh(),
        ),
        data: (state) {
          if (state.error != null) {
            return _ErrorView(
              error: state.error!,
              onRetry: () =>
                  ref.read(webhookDetailProvider(webhookId).notifier).refresh(),
            );
          }
          final webhook = state.webhook;
          if (webhook == null) {
            return const Center(child: Text('Webhook not found'));
          }
          return _WebhookDetailBody(
            webhook: webhook,
            actionMessage: state.actionMessage,
            onVerify: () =>
                ref.read(webhookDetailProvider(webhookId).notifier).verify(),
            onRotateSecret: () => ref
                .read(webhookDetailProvider(webhookId).notifier)
                .rotateSecret(),
          );
        },
      ),
    );
  }
}

class _WebhookDetailBody extends StatelessWidget {
  const _WebhookDetailBody({
    required this.webhook,
    required this.onVerify,
    required this.onRotateSecret,
    this.actionMessage,
  });

  final WebhookResponse webhook;
  final VoidCallback onVerify;
  final VoidCallback onRotateSecret;
  final String? actionMessage;

  @override
  Widget build(BuildContext context) {
    final isVerified = webhook.verifiedAt != null;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        if (actionMessage != null)
          Padding(
            padding: const EdgeInsets.only(bottom: 16),
            child: Material(
              color: Colors.green.shade50,
              borderRadius: BorderRadius.circular(8),
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Row(
                  children: [
                    Icon(Icons.check_circle_outline,
                        color: Colors.green.shade700, size: 18),
                    const SizedBox(width: 8),
                    Text(
                      actionMessage!,
                      style: TextStyle(color: Colors.green.shade700),
                    ),
                  ],
                ),
              ),
            ),
          ),

        // Header card.
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        webhook.name,
                        style: Theme.of(context).textTheme.headlineSmall,
                      ),
                    ),
                    Container(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 8, vertical: 3),
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
                const SizedBox(height: 8),
                _InfoRow(label: 'URL', value: webhook.url),
                const SizedBox(height: 4),
                _InfoRow(
                  label: 'Event types',
                  value: webhook.eventTypes.join(', '),
                ),
                const SizedBox(height: 4),
                _InfoRow(
                  label: 'Verified',
                  value: isVerified
                      ? 'Yes — ${webhook.verifiedAt}'
                      : 'Not verified',
                ),
              ],
            ),
          ),
        ),

        const SizedBox(height: 16),

        // Actions.
        Text(
          'Actions',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        OutlinedButton.icon(
          onPressed: onVerify,
          icon: const Icon(Icons.verified_outlined),
          label: const Text('Verify webhook'),
        ),
        const SizedBox(height: 8),
        OutlinedButton.icon(
          onPressed: onRotateSecret,
          icon: const Icon(Icons.key_outlined),
          label: const Text('Rotate secret'),
          style: OutlinedButton.styleFrom(
            foregroundColor: Colors.orange.shade700,
          ),
        ),
      ],
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 100,
          child: Text(
            '$label:',
            style: const TextStyle(
              fontSize: 12,
              fontWeight: FontWeight.w600,
              color: Colors.black54,
            ),
          ),
        ),
        Expanded(
          child: Text(
            value,
            style: const TextStyle(fontSize: 12),
          ),
        ),
      ],
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
