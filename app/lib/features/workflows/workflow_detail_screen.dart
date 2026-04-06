import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'workflows_provider.dart';

/// Screen that shows the full detail of a single workflow.
class WorkflowDetailScreen extends ConsumerWidget {
  const WorkflowDetailScreen({super.key, required this.workflowId});

  final String workflowId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final detailAsync = ref.watch(workflowDetailProvider(workflowId));

    return Scaffold(
      appBar: AppBar(
        title: Text(
          detailAsync.value?.detail?.name ?? 'Workflow',
        ),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/workflows'),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () =>
                ref.read(workflowDetailProvider(workflowId).notifier).refresh(),
          ),
        ],
      ),
      body: detailAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () =>
              ref.read(workflowDetailProvider(workflowId).notifier).refresh(),
        ),
        data: (state) {
          if (state.error != null) {
            return _ErrorView(
              error: state.error!,
              onRetry: () => ref
                  .read(workflowDetailProvider(workflowId).notifier)
                  .refresh(),
            );
          }
          final detail = state.detail;
          if (detail == null) {
            return const Center(child: Text('Workflow not found'));
          }
          return _WorkflowDetailBody(detail: detail, runs: state.runs);
        },
      ),
    );
  }
}

class _WorkflowDetailBody extends StatelessWidget {
  const _WorkflowDetailBody({
    required this.detail,
    required this.runs,
  });

  final WorkflowDetail detail;
  final List<WorkflowRun> runs;

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
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 3,
                      ),
                      decoration: BoxDecoration(
                        color: detail.active
                            ? Colors.green.shade50
                            : Colors.grey.shade100,
                        borderRadius: BorderRadius.circular(12),
                        border: Border.all(
                          color: detail.active
                              ? Colors.green.shade300
                              : Colors.grey.shade400,
                        ),
                      ),
                      child: Text(
                        detail.active ? 'Active' : 'Inactive',
                        style: TextStyle(
                          fontSize: 11,
                          fontWeight: FontWeight.w600,
                          color: detail.active
                              ? Colors.green.shade700
                              : Colors.grey.shade600,
                        ),
                      ),
                    ),
                  ],
                ),
                if (detail.description.isNotEmpty) ...[
                  const SizedBox(height: 8),
                  Text(
                    detail.description,
                    style: const TextStyle(color: Colors.black54),
                  ),
                ],
                if (detail.webhookUrl != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    'Webhook: ${detail.webhookUrl}',
                    style: const TextStyle(
                      fontSize: 12,
                      fontFamily: 'monospace',
                      color: Colors.black54,
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),

        const SizedBox(height: 16),

        // Recent runs.
        Text(
          'Recent Runs',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),

        if (runs.isEmpty)
          const Center(
            child: Padding(
              padding: EdgeInsets.symmetric(vertical: 24),
              child: Text(
                'No runs yet',
                style: TextStyle(color: Colors.black38),
              ),
            ),
          )
        else
          ...runs.map((run) => _RunTile(run: run)),
      ],
    );
  }
}

class _RunTile extends StatelessWidget {
  const _RunTile({required this.run});

  final WorkflowRun run;

  @override
  Widget build(BuildContext context) {
    final isSuccess = run.status == 'completed' || run.status == 'success';
    final isError = run.status == 'failed' || run.status == 'error';

    return Card(
      margin: const EdgeInsets.only(bottom: 4),
      child: ListTile(
        leading: Icon(
          isSuccess
              ? Icons.check_circle_outline
              : isError
                  ? Icons.error_outline
                  : Icons.hourglass_empty,
          color: isSuccess
              ? Colors.green.shade600
              : isError
                  ? Colors.red.shade600
                  : Colors.orange.shade600,
        ),
        title: Text(
          'Run ${run.id.length > 8 ? run.id.substring(0, 8) : run.id}...',
          style: const TextStyle(fontFamily: 'monospace', fontSize: 13),
        ),
        subtitle: Text(
          run.startedAt,
          style: const TextStyle(fontSize: 11, color: Colors.black54),
        ),
        trailing: Container(
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
          decoration: BoxDecoration(
            color: isSuccess
                ? Colors.green.shade50
                : isError
                    ? Colors.red.shade50
                    : Colors.orange.shade50,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: isSuccess
                  ? Colors.green.shade300
                  : isError
                      ? Colors.red.shade300
                      : Colors.orange.shade300,
            ),
          ),
          child: Text(
            run.status,
            style: TextStyle(
              fontSize: 10,
              fontWeight: FontWeight.w600,
              color: isSuccess
                  ? Colors.green.shade700
                  : isError
                      ? Colors.red.shade700
                      : Colors.orange.shade700,
            ),
          ),
        ),
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
