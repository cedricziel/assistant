import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'workflows_provider.dart';

/// Full detail view for a single workflow run, showing all execution steps.
class WorkflowRunDetailScreen extends ConsumerWidget {
  const WorkflowRunDetailScreen({
    super.key,
    required this.workflowId,
    required this.runId,
  });

  final String workflowId;
  final String runId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final key = (workflowId: workflowId, runId: runId);
    final detailAsync = ref.watch(workflowRunDetailProvider(key));

    return Scaffold(
      appBar: AppBar(
        title: const Text('Run Detail'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/workflows/$workflowId'),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () =>
                ref.read(workflowRunDetailProvider(key).notifier).refresh(),
          ),
        ],
      ),
      body: detailAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () =>
              ref.read(workflowRunDetailProvider(key).notifier).refresh(),
        ),
        data: (state) {
          if (state.error != null) {
            return _ErrorView(
              error: state.error!,
              onRetry: () =>
                  ref.read(workflowRunDetailProvider(key).notifier).refresh(),
            );
          }
          final detail = state.runDetail;
          if (detail == null) {
            return const Center(child: Text('Run not found'));
          }
          return _RunDetailBody(detail: detail);
        },
      ),
    );
  }
}

class _RunDetailBody extends StatelessWidget {
  const _RunDetailBody({required this.detail});

  final WorkflowRunDetail detail;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final run = detail.run;
    final steps = detail.steps.toList()
      ..sort((a, b) => a.stepIndex.compareTo(b.stepIndex));

    final isSuccess = run.status == 'completed' || run.status == 'success';
    final isError = run.status == 'failed' || run.status == 'error';

    final statusColor = isSuccess
        ? Colors.green.shade600
        : isError
            ? Colors.red.shade600
            : Colors.orange.shade600;

    final statusBg = isSuccess
        ? Colors.green.shade50
        : isError
            ? Colors.red.shade50
            : Colors.orange.shade50;

    final statusBorder = isSuccess
        ? Colors.green.shade300
        : isError
            ? Colors.red.shade300
            : Colors.orange.shade300;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // Summary card
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: SelectableText(
                        'Run ${run.id.length > 12 ? run.id.substring(0, 12) : run.id}…',
                        style: const TextStyle(
                          fontFamily: 'monospace',
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 3,
                      ),
                      decoration: BoxDecoration(
                        color: statusBg,
                        borderRadius: BorderRadius.circular(12),
                        border: Border.all(color: statusBorder),
                      ),
                      child: Text(
                        run.status,
                        style: TextStyle(
                          fontSize: 11,
                          fontWeight: FontWeight.w600,
                          color: statusColor,
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                _InfoRow(
                  label: 'Trigger',
                  value: run.triggerType,
                  icon: Icons.bolt,
                ),
                _InfoRow(
                  label: 'Started',
                  value: _formatTimestamp(run.startedAt),
                  icon: Icons.schedule,
                ),
                if (run.finishedAt != null)
                  _InfoRow(
                    label: 'Finished',
                    value: _formatTimestamp(run.finishedAt!),
                    icon: Icons.check_circle_outline,
                  ),
                _InfoRow(
                  label: 'Steps',
                  value: '${steps.length}',
                  icon: Icons.list_alt,
                ),
                if (run.errorMessage != null) ...[
                  const SizedBox(height: 8),
                  Container(
                    padding: const EdgeInsets.all(10),
                    decoration: BoxDecoration(
                      color: Colors.red.shade50,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: Colors.red.shade200),
                    ),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Icon(
                          Icons.error_outline,
                          size: 16,
                          color: Colors.red.shade600,
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            run.errorMessage!,
                            style: TextStyle(
                              fontSize: 12,
                              color: Colors.red.shade700,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),

        const SizedBox(height: 16),

        Text(
          'Execution Steps',
          style: theme.textTheme.titleMedium,
        ),
        const SizedBox(height: 8),

        if (steps.isEmpty)
          const Center(
            child: Padding(
              padding: EdgeInsets.symmetric(vertical: 24),
              child: Text(
                'No steps recorded',
                style: TextStyle(color: Colors.black45),
              ),
            ),
          )
        else
          ...steps.asMap().entries.map(
                (entry) => _StepTile(
                  step: entry.value,
                  isLast: entry.key == steps.length - 1,
                ),
              ),
      ],
    );
  }

  String _formatTimestamp(DateTime dt) {
    return '${dt.year}-'
        '${dt.month.toString().padLeft(2, '0')}-'
        '${dt.day.toString().padLeft(2, '0')} '
        '${dt.hour.toString().padLeft(2, '0')}:'
        '${dt.minute.toString().padLeft(2, '0')}:'
        '${dt.second.toString().padLeft(2, '0')}';
  }
}

class _StepTile extends StatefulWidget {
  const _StepTile({required this.step, required this.isLast});

  final WorkflowRunStep step;
  final bool isLast;

  @override
  State<_StepTile> createState() => _StepTileState();
}

class _StepTileState extends State<_StepTile> {
  bool _expanded = false;

  Color _kindColor(String kind) {
    switch (kind) {
      case 'trigger':
        return Colors.blue.shade600;
      case 'action':
        return Colors.green.shade600;
      case 'condition':
        return Colors.orange.shade600;
      default:
        return Colors.grey.shade600;
    }
  }

  IconData _kindIcon(String kind) {
    switch (kind) {
      case 'trigger':
        return Icons.play_circle_outline;
      case 'action':
        return Icons.bolt;
      case 'condition':
        return Icons.call_split;
      default:
        return Icons.circle_outlined;
    }
  }

  @override
  Widget build(BuildContext context) {
    final step = widget.step;
    final kindColor = _kindColor(step.nodeKind);
    final hasOutput = step.output != null;
    final hasNote = step.note != null && step.note!.isNotEmpty;

    return IntrinsicHeight(
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Timeline column
          SizedBox(
            width: 36,
            child: Column(
              children: [
                Container(
                  width: 28,
                  height: 28,
                  decoration: BoxDecoration(
                    color: kindColor.withValues(alpha: 0.15),
                    shape: BoxShape.circle,
                    border: Border.all(color: kindColor, width: 1.5),
                  ),
                  child: Icon(_kindIcon(step.nodeKind), size: 14, color: kindColor),
                ),
                if (!widget.isLast)
                  Expanded(
                    child: Container(
                      width: 2,
                      color: Colors.black12,
                    ),
                  ),
              ],
            ),
          ),

          const SizedBox(width: 8),

          // Content
          Expanded(
            child: Padding(
              padding: const EdgeInsets.only(bottom: 12),
              child: Card(
                margin: EdgeInsets.zero,
                child: InkWell(
                  onTap: (hasOutput || hasNote)
                      ? () => setState(() => _expanded = !_expanded)
                      : null,
                  borderRadius: BorderRadius.circular(12),
                  child: Padding(
                    padding: const EdgeInsets.all(10),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 6,
                                vertical: 2,
                              ),
                              decoration: BoxDecoration(
                                color: kindColor.withValues(alpha: 0.12),
                                borderRadius: BorderRadius.circular(4),
                              ),
                              child: Text(
                                step.nodeKind,
                                style: TextStyle(
                                  fontSize: 10,
                                  fontWeight: FontWeight.w700,
                                  color: kindColor,
                                  fontFamily: 'monospace',
                                ),
                              ),
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                step.nodeId,
                                style: const TextStyle(
                                  fontFamily: 'monospace',
                                  fontSize: 12,
                                  fontWeight: FontWeight.w600,
                                ),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                            Text(
                              '#${step.stepIndex}',
                              style: const TextStyle(
                                fontSize: 11,
                                color: Colors.black38,
                              ),
                            ),
                            if (hasOutput || hasNote)
                              Icon(
                                _expanded
                                    ? Icons.keyboard_arrow_up
                                    : Icons.keyboard_arrow_down,
                                size: 14,
                                color: Colors.black38,
                              ),
                          ],
                        ),
                        if (hasNote) ...[
                          const SizedBox(height: 4),
                          Text(
                            step.note!,
                            style: const TextStyle(
                              fontSize: 12,
                              color: Colors.black54,
                            ),
                          ),
                        ],
                        if (_expanded && hasOutput) ...[
                          const SizedBox(height: 6),
                          const Divider(height: 1),
                          const SizedBox(height: 6),
                          Container(
                            width: double.infinity,
                            padding: const EdgeInsets.all(8),
                            decoration: BoxDecoration(
                              color: Colors.black.withValues(alpha: 0.04),
                              borderRadius: BorderRadius.circular(6),
                            ),
                            child: SelectableText(
                              step.output?.value.toString() ?? 'null',
                              style: const TextStyle(
                                fontFamily: 'monospace',
                                fontSize: 11,
                              ),
                            ),
                          ),
                        ],
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({
    required this.label,
    required this.value,
    required this.icon,
  });

  final String label;
  final String value;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        children: [
          Icon(icon, size: 14, color: Colors.black38),
          const SizedBox(width: 6),
          SizedBox(
            width: 60,
            child: Text(
              label,
              style: const TextStyle(fontSize: 12, color: Colors.black45),
            ),
          ),
          Expanded(
            child: Text(
              value,
              style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w500),
            ),
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
