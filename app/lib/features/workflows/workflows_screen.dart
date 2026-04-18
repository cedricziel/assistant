import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'workflows_provider.dart';

/// Screen that lists all workflows.
class WorkflowsScreen extends ConsumerWidget {
  const WorkflowsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final workflowsAsync = ref.watch(workflowsProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Workflows'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => ref.read(workflowsProvider.notifier).refresh(),
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: () => context.go('/workflows/new'),
        tooltip: 'New workflow',
        child: const Icon(Icons.add),
      ),
      body: workflowsAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () => ref.read(workflowsProvider.notifier).refresh(),
        ),
        data: (workflows) {
          if (workflows.isEmpty) return const _EmptyView();
          return ListView.separated(
            padding: const EdgeInsets.symmetric(vertical: 8),
            itemCount: workflows.length,
            separatorBuilder: (_, _) => const Divider(height: 1, indent: 72),
            itemBuilder: (context, index) =>
                _WorkflowRow(workflow: workflows[index]),
          );
        },
      ),
    );
  }
}

class _WorkflowRow extends StatelessWidget {
  const _WorkflowRow({required this.workflow});

  final WorkflowSummary workflow;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final badge = Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: workflow.active
            ? Colors.green.shade50
            : colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: workflow.active ? Colors.green.shade300 : Colors.grey.shade400,
        ),
      ),
      child: Text(
        workflow.active ? 'Active' : 'Inactive',
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          color: workflow.active ? Colors.green.shade700 : Colors.grey.shade600,
        ),
      ),
    );

    return ListTile(
      leading: CircleAvatar(
        backgroundColor: workflow.active
            ? Colors.green.shade100
            : Colors.grey.shade200,
        child: Icon(
          Icons.account_tree_outlined,
          size: 20,
          color: workflow.active ? Colors.green.shade700 : Colors.grey.shade500,
        ),
      ),
      title: Row(
        children: [
          Expanded(
            child: Text(
              workflow.name,
              style: const TextStyle(fontWeight: FontWeight.w600),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          const SizedBox(width: 8),
          badge,
        ],
      ),
      subtitle: workflow.description.isNotEmpty
          ? Text(
              workflow.description,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontSize: 12),
            )
          : null,
      onTap: () => context.go('/workflows/${workflow.id}'),
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
          Icon(
            Icons.account_tree_outlined,
            size: 64,
            color: colorScheme.outlineVariant,
          ),
          const SizedBox(height: 12),
          Text(
            'No workflows yet',
            style: TextStyle(fontSize: 16, color: colorScheme.onSurfaceVariant),
          ),
          const SizedBox(height: 4),
          Text(
            'Create a workflow to automate tasks.',
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
          FilledButton(onPressed: onRetry, child: const Text('Retry')),
        ],
      ),
    );
  }
}
