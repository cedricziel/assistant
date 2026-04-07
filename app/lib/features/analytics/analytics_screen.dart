import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'analytics_provider.dart';

/// Screen showing aggregated assistant usage analytics.
class AnalyticsScreen extends ConsumerWidget {
  const AnalyticsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final analyticsAsync = ref.watch(analyticsProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Analytics'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => ref.read(analyticsProvider.notifier).refresh(),
          ),
        ],
      ),
      body: analyticsAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () => ref.read(analyticsProvider.notifier).refresh(),
        ),
        data: (state) {
          if (state.error != null) {
            return _ErrorView(
              error: state.error!,
              onRetry: () => ref.read(analyticsProvider.notifier).refresh(),
            );
          }
          return _AnalyticsBody(state: state);
        },
      ),
    );
  }
}

// -- Time window selector ----------------------------------------------------

const _windows = [
  (hours: 1, label: '1h'),
  (hours: 6, label: '6h'),
  (hours: 24, label: '24h'),
  (hours: 72, label: '3d'),
  (hours: 168, label: '7d'),
];

class _AnalyticsBody extends ConsumerWidget {
  const _AnalyticsBody({required this.state});

  final AnalyticsState state;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final summary = state.summary;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // Time window selector.
        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Row(
            children: _windows.map((w) {
              final selected = state.windowHours == w.hours;
              return Padding(
                padding: const EdgeInsets.only(right: 8),
                child: ChoiceChip(
                  label: Text(w.label),
                  selected: selected,
                  onSelected: (v) {
                    if (v) {
                      ref.read(analyticsProvider.notifier).setWindow(w.hours);
                    }
                  },
                ),
              );
            }).toList(),
          ),
        ),

        const SizedBox(height: 16),

        if (summary == null)
          const Center(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.bar_chart_outlined,
                    size: 64, color: Colors.black26),
                SizedBox(height: 12),
                Text(
                  'No analytics data',
                  style: TextStyle(fontSize: 16, color: Colors.black45),
                ),
                SizedBox(height: 4),
                Text(
                  'Use the assistant to generate analytics.',
                  style: TextStyle(color: Colors.black38),
                ),
              ],
            ),
          )
        else ...[
          // Summary cards.
          _SummaryCards(summary: summary),
          const SizedBox(height: 16),

          // Model breakdown.
          Text(
            'Model Breakdown',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          _ModelTable(summary: summary),
          const SizedBox(height: 16),

          // Tool usage.
          Text(
            'Tool Usage',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          _ToolTable(summary: summary),
        ],
      ],
    );
  }
}

class _SummaryCards extends StatelessWidget {
  const _SummaryCards({required this.summary});

  final AnalyticsSummaryResponse summary;

  @override
  Widget build(BuildContext context) {
    return GridView.count(
      crossAxisCount: 2,
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      mainAxisSpacing: 8,
      crossAxisSpacing: 8,
      childAspectRatio: 2.5,
      children: [
        _StatCard(
          label: 'Requests',
          value: summary.totalRequests.toString(),
          icon: Icons.send_outlined,
        ),
        _StatCard(
          label: 'Tokens In',
          value: _formatCount(summary.totalTokensIn),
          icon: Icons.arrow_downward_outlined,
        ),
        _StatCard(
          label: 'Tokens Out',
          value: _formatCount(summary.totalTokensOut),
          icon: Icons.arrow_upward_outlined,
        ),
        _StatCard(
          label: 'Tool Calls',
          value: summary.totalToolInvocations.toString(),
          icon: Icons.extension_outlined,
        ),
        _StatCard(
          label: 'Errors',
          value: summary.errorCount.toString(),
          icon: Icons.error_outline,
          valueColor:
              summary.errorCount > 0 ? Colors.red.shade700 : null,
        ),
        _StatCard(
          label: 'Avg Duration',
          value: '${summary.avgDurationS.toStringAsFixed(1)}s',
          icon: Icons.timer_outlined,
        ),
      ],
    );
  }

  String _formatCount(int n) {
    if (n >= 1000000) return '${(n / 1000000).toStringAsFixed(1)}M';
    if (n >= 1000) return '${(n / 1000).toStringAsFixed(1)}k';
    return n.toString();
  }
}

class _StatCard extends StatelessWidget {
  const _StatCard({
    required this.label,
    required this.value,
    required this.icon,
    this.valueColor,
  });

  final String label;
  final String value;
  final IconData icon;
  final Color? valueColor;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        child: Row(
          children: [
            Icon(icon, size: 20, color: Colors.black38),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Text(
                    value,
                    style: TextStyle(
                      fontSize: 18,
                      fontWeight: FontWeight.w700,
                      color: valueColor,
                    ),
                  ),
                  Text(
                    label,
                    style: const TextStyle(
                      fontSize: 11,
                      color: Colors.black54,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ModelTable extends StatelessWidget {
  const _ModelTable({required this.summary});

  final AnalyticsSummaryResponse summary;

  @override
  Widget build(BuildContext context) {
    if (summary.models.isEmpty) {
      return const Text(
        'No model data',
        style: TextStyle(color: Colors.black38),
      );
    }

    return Card(
      child: DataTable(
        columnSpacing: 16,
        columns: const [
          DataColumn(label: Text('Model')),
          DataColumn(label: Text('Requests'), numeric: true),
          DataColumn(label: Text('Tokens In'), numeric: true),
          DataColumn(label: Text('Tokens Out'), numeric: true),
        ],
        rows: summary.models
            .map(
              (m) => DataRow(cells: [
                DataCell(Text(
                  m.model,
                  style: const TextStyle(
                    fontSize: 12,
                    fontFamily: 'monospace',
                  ),
                )),
                DataCell(Text(m.requestCount.toString())),
                DataCell(Text(m.inputTokens.toString())),
                DataCell(Text(m.outputTokens.toString())),
              ]),
            )
            .toList(),
      ),
    );
  }
}

class _ToolTable extends StatelessWidget {
  const _ToolTable({required this.summary});

  final AnalyticsSummaryResponse summary;

  @override
  Widget build(BuildContext context) {
    if (summary.tools.isEmpty) {
      return const Text(
        'No tool usage data',
        style: TextStyle(color: Colors.black38),
      );
    }

    return Card(
      child: DataTable(
        columnSpacing: 16,
        columns: const [
          DataColumn(label: Text('Tool')),
          DataColumn(label: Text('Count'), numeric: true),
        ],
        rows: summary.tools
            .map(
              (t) => DataRow(cells: [
                DataCell(Text(
                  t.toolName,
                  style: const TextStyle(
                    fontSize: 12,
                    fontFamily: 'monospace',
                  ),
                )),
                DataCell(Text(t.invocations.toString())),
              ]),
            )
            .toList(),
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
