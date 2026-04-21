import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart' show Clipboard, ClipboardData;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'workflows_provider.dart';

/// Screen that shows the full detail of a single workflow.
class WorkflowDetailScreen extends ConsumerStatefulWidget {
  const WorkflowDetailScreen({super.key, required this.workflowId});

  final String workflowId;

  @override
  ConsumerState<WorkflowDetailScreen> createState() =>
      _WorkflowDetailScreenState();
}

class _WorkflowDetailScreenState extends ConsumerState<WorkflowDetailScreen> {
  WorkflowWebhookSecrets? _webhookSecrets;
  bool _webhookSecretsFetched = false;

  /// Detects whether the workflow graph includes a webhook trigger node.
  bool _hasWebhookTrigger(WorkflowDetail detail) {
    final graphRaw = detail.graph?.value;
    if (graphRaw is! Map) return false;
    final nodes = graphRaw['nodes'] as List? ?? [];
    return nodes.any((n) {
      if (n is! Map) return false;
      final config = n['config'] as Map? ?? {};
      return config['type'] == 'webhook';
    });
  }

  Future<void> _fetchWebhookSecrets() async {
    if (_webhookSecretsFetched) return;
    _webhookSecretsFetched = true;
    final notifier = ref.read(
      workflowDetailProvider(widget.workflowId).notifier,
    );
    final secrets = await notifier.fetchWebhookSecrets();
    if (mounted) setState(() => _webhookSecrets = secrets);
  }

  Future<void> _confirmDelete() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Delete workflow?'),
        content: const Text(
          'This will permanently delete the workflow and all its run history.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
            ),
            onPressed: () => Navigator.of(ctx).pop(true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;

    final error = await ref
        .read(workflowsProvider.notifier)
        .deleteWorkflow(widget.workflowId);
    if (!mounted) return;
    if (error != null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Delete failed: $error'),
          backgroundColor: Theme.of(context).colorScheme.error,
          behavior: SnackBarBehavior.floating,
        ),
      );
      return;
    }
    context.go('/workflows');
  }

  Future<void> _toggleActive(bool currentlyActive) async {
    final notifier = ref.read(workflowsProvider.notifier);
    final error = currentlyActive
        ? await notifier.deactivateWorkflow(widget.workflowId)
        : await notifier.activateWorkflow(widget.workflowId);

    if (!mounted) return;
    if (error != null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Failed: $error'),
          backgroundColor: Theme.of(context).colorScheme.error,
          behavior: SnackBarBehavior.floating,
        ),
      );
      return;
    }
    ref.read(workflowDetailProvider(widget.workflowId).notifier).refresh();
  }

  Future<void> _testRun() async {
    final notifier = ref.read(
      workflowDetailProvider(widget.workflowId).notifier,
    );
    final (preview, error) = await notifier.testRun();
    if (!mounted) return;
    if (error != null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Test run failed: $error'),
          backgroundColor: Theme.of(context).colorScheme.error,
          behavior: SnackBarBehavior.floating,
        ),
      );
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('Test run started: ${preview!.message}'),
        behavior: SnackBarBehavior.floating,
        action: SnackBarAction(
          label: 'View',
          onPressed: () => context.go(
            '/workflows/${widget.workflowId}/runs/${preview.runId}',
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final detailAsync = ref.watch(workflowDetailProvider(widget.workflowId));

    return Scaffold(
      appBar: AppBar(
        title: Text(detailAsync.value?.detail.name ?? 'Workflow'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/workflows'),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.play_arrow),
            tooltip: 'Test run',
            onPressed: _testRun,
          ),
          IconButton(
            icon: const Icon(Icons.edit_outlined),
            tooltip: 'Edit workflow',
            onPressed: () => context.go('/workflows/${widget.workflowId}/edit'),
          ),
          IconButton(
            icon: Icon(
              Icons.delete_outline,
              color: Theme.of(context).colorScheme.error,
            ),
            tooltip: 'Delete workflow',
            onPressed: _confirmDelete,
          ),
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => ref
                .read(workflowDetailProvider(widget.workflowId).notifier)
                .refresh(),
          ),
        ],
      ),
      body: detailAsync.when(
        loading: () =>
            const Center(child: CircularProgressIndicator.adaptive()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () => ref
              .read(workflowDetailProvider(widget.workflowId).notifier)
              .refresh(),
        ),
        data: (state) {
          // Fetch webhook secrets lazily when we know it's a webhook workflow.
          if (_hasWebhookTrigger(state.detail)) {
            _fetchWebhookSecrets();
          }
          return _WorkflowDetailBody(
            workflowId: widget.workflowId,
            detail: state.detail,
            runs: state.runs,
            onToggleActive: () => _toggleActive(state.detail.active),
            webhookSecrets: _webhookSecrets,
          );
        },
      ),
    );
  }
}

class _WorkflowDetailBody extends StatelessWidget {
  const _WorkflowDetailBody({
    required this.workflowId,
    required this.detail,
    required this.runs,
    required this.onToggleActive,
    this.webhookSecrets,
  });

  final String workflowId;
  final WorkflowDetail detail;
  final List<WorkflowRunSummary> runs;
  final VoidCallback onToggleActive;
  final WorkflowWebhookSecrets? webhookSecrets;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
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
                        detail.name,
                        style: Theme.of(context).textTheme.headlineSmall,
                      ),
                    ),
                    // Active toggle chip
                    GestureDetector(
                      onTap: onToggleActive,
                      child: Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 3,
                        ),
                        decoration: BoxDecoration(
                          color: detail.active
                              ? Colors.green.withAlpha(20)
                              : Colors.grey.withAlpha(20),
                          borderRadius: BorderRadius.circular(12),
                          border: Border.all(
                            color: detail.active
                                ? Colors.green.withAlpha(60)
                                : Colors.grey.withAlpha(60),
                          ),
                        ),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(
                              detail.active
                                  ? Icons.pause_circle_outline
                                  : Icons.play_circle_outline,
                              size: 12,
                              color: detail.active ? Colors.green : Colors.grey,
                            ),
                            const SizedBox(width: 4),
                            Text(
                              detail.active ? 'Active' : 'Inactive',
                              style: TextStyle(
                                fontSize: 11,
                                fontWeight: FontWeight.w600,
                                color: detail.active
                                    ? Colors.green
                                    : Colors.grey,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ],
                ),
                if (detail.description.isNotEmpty) ...[
                  const SizedBox(height: 8),
                  Text(
                    detail.description,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),

        const SizedBox(height: 16),

        // Webhook secrets card (shown only for webhook-triggered workflows).
        if (webhookSecrets != null) ...[
          _WebhookSecretsCard(secrets: webhookSecrets!),
          const SizedBox(height: 16),
        ],

        // Recent runs.
        Text('Recent Runs', style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 8),

        if (runs.isEmpty)
          Center(
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 24),
              child: Text(
                'No runs yet',
                style: TextStyle(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          )
        else ...[
          ...runs
              .take(20)
              .map((run) => _RunTile(workflowId: workflowId, run: run)),
          if (runs.length > 20)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 8),
              child: Text(
                'Showing 20 of ${runs.length} runs',
                textAlign: TextAlign.center,
                style: TextStyle(
                  fontSize: 12,
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ),
        ],
      ],
    );
  }
}

class _RunTile extends StatelessWidget {
  const _RunTile({required this.workflowId, required this.run});

  final String workflowId;
  final WorkflowRunSummary run;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final status = run.status;
    final isSuccess = status == 'completed' || status == 'success';
    final isError = status == 'failed' || status == 'error';

    return Card(
      margin: const EdgeInsets.only(bottom: 4),
      child: ListTile(
        onTap: () => context.go('/workflows/$workflowId/runs/${run.id}'),
        leading: Icon(
          isSuccess
              ? Icons.check_circle_outline
              : isError
              ? Icons.error_outline
              : Icons.hourglass_empty,
          color: isSuccess
              ? Colors.green
              : isError
              ? colorScheme.error
              : Colors.orange,
        ),
        title: Text(
          'Run ${run.id.length > 8 ? run.id.substring(0, 8) : run.id}…',
          style: const TextStyle(fontFamily: 'monospace', fontSize: 13),
        ),
        subtitle: Text(
          run.startedAt.toIso8601String(),
          style: TextStyle(fontSize: 11, color: colorScheme.onSurfaceVariant),
        ),
        trailing: Container(
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
          decoration: BoxDecoration(
            color: isSuccess
                ? Colors.green.withAlpha(20)
                : isError
                ? colorScheme.errorContainer
                : Colors.orange.withAlpha(20),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: isSuccess
                  ? Colors.green.withAlpha(60)
                  : isError
                  ? colorScheme.error
                  : Colors.orange.withAlpha(60),
            ),
          ),
          child: Text(
            status,
            style: TextStyle(
              fontSize: 10,
              fontWeight: FontWeight.w600,
              color: isSuccess
                  ? Colors.green
                  : isError
                  ? colorScheme.error
                  : Colors.orange,
            ),
          ),
        ),
      ),
    );
  }
}

class _WebhookSecretsCard extends StatefulWidget {
  const _WebhookSecretsCard({required this.secrets});

  final WorkflowWebhookSecrets secrets;

  @override
  State<_WebhookSecretsCard> createState() => _WebhookSecretsCardState();
}

class _WebhookSecretsCardState extends State<_WebhookSecretsCard> {
  bool _tokenVisible = false;

  void _copy(String value, String label) {
    Clipboard.setData(ClipboardData(text: value));
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('$label copied to clipboard'),
        behavior: SnackBarBehavior.floating,
        duration: const Duration(seconds: 2),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.webhook, color: Colors.blue, size: 20),
                const SizedBox(width: 8),
                Text('Webhook', style: theme.textTheme.titleSmall),
              ],
            ),
            const SizedBox(height: 12),
            // URL
            _CopyableField(
              label: 'URL',
              value: widget.secrets.webhookUrl,
              onCopy: () => _copy(widget.secrets.webhookUrl, 'URL'),
            ),
            const SizedBox(height: 8),
            // Token
            Row(
              children: [
                SizedBox(
                  width: 48,
                  child: Text(
                    'Token',
                    style: TextStyle(
                      fontSize: 12,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                Expanded(
                  child: Text(
                    _tokenVisible ? widget.secrets.webhookToken : '\u2022' * 16,
                    style: const TextStyle(
                      fontSize: 12,
                      fontFamily: 'monospace',
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                IconButton(
                  icon: Icon(
                    _tokenVisible ? Icons.visibility_off : Icons.visibility,
                    size: 16,
                  ),
                  onPressed: () =>
                      setState(() => _tokenVisible = !_tokenVisible),
                  tooltip: _tokenVisible ? 'Hide' : 'Show',
                  visualDensity: VisualDensity.compact,
                ),
                IconButton(
                  icon: const Icon(Icons.copy, size: 16),
                  onPressed: () => _copy(widget.secrets.webhookToken, 'Token'),
                  tooltip: 'Copy token',
                  visualDensity: VisualDensity.compact,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _CopyableField extends StatelessWidget {
  const _CopyableField({
    required this.label,
    required this.value,
    required this.onCopy,
  });

  final String label;
  final String value;
  final VoidCallback onCopy;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        SizedBox(
          width: 48,
          child: Text(
            label,
            style: TextStyle(
              fontSize: 12,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        ),
        Expanded(
          child: Text(
            value,
            style: const TextStyle(fontSize: 12, fontFamily: 'monospace'),
            overflow: TextOverflow.ellipsis,
          ),
        ),
        IconButton(
          icon: const Icon(Icons.copy, size: 16),
          onPressed: onCopy,
          tooltip: 'Copy $label',
          visualDensity: VisualDensity.compact,
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
          Icon(
            Icons.error_outline,
            color: Theme.of(context).colorScheme.error,
            size: 48,
          ),
          const SizedBox(height: 12),
          Text(error, textAlign: TextAlign.center),
          const SizedBox(height: 12),
          FilledButton(onPressed: onRetry, child: const Text('Retry')),
        ],
      ),
    );
  }
}
