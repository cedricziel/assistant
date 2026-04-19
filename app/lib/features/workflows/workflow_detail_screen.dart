import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
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
        data: (state) => _WorkflowDetailBody(
          workflowId: widget.workflowId,
          detail: state.detail,
          runs: state.runs,
          onToggleActive: () => _toggleActive(state.detail.active),
        ),
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
  });

  final String workflowId;
  final WorkflowDetail detail;
  final List<WorkflowRunSummary> runs;
  final VoidCallback onToggleActive;

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
                              ? Colors.green.shade50
                              : Colors.grey.shade100,
                          borderRadius: BorderRadius.circular(12),
                          border: Border.all(
                            color: detail.active
                                ? Colors.green.shade300
                                : Colors.grey.shade400,
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
                              color: detail.active
                                  ? Colors.green.shade700
                                  : Colors.grey.shade600,
                            ),
                            const SizedBox(width: 4),
                            Text(
                              detail.active ? 'Active' : 'Inactive',
                              style: TextStyle(
                                fontSize: 11,
                                fontWeight: FontWeight.w600,
                                color: detail.active
                                    ? Colors.green.shade700
                                    : Colors.grey.shade600,
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
              ? Colors.green.shade600
              : isError
              ? colorScheme.error
              : Colors.orange.shade600,
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
                ? Colors.green.shade50
                : isError
                ? colorScheme.errorContainer
                : Colors.orange.shade50,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: isSuccess
                  ? Colors.green.shade300
                  : isError
                  ? colorScheme.error
                  : Colors.orange.shade300,
            ),
          ),
          child: Text(
            status,
            style: TextStyle(
              fontSize: 10,
              fontWeight: FontWeight.w600,
              color: isSuccess
                  ? Colors.green.shade700
                  : isError
                  ? colorScheme.error
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
