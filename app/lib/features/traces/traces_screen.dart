import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_api/assistant_api.dart';

import 'traces_provider.dart';

/// Observability screen that lists recent assistant traces.
///
/// Each row shows timestamp, persona, duration, and status.
/// Expanding a row reveals the span breakdown.
class TracesScreen extends ConsumerWidget {
  const TracesScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tracesAsync = ref.watch(tracesProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Traces'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/chat'),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => ref.read(tracesProvider.notifier).refresh(),
          ),
        ],
      ),
      body: tracesAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () => ref.read(tracesProvider.notifier).refresh(),
        ),
        data: (state) {
          if (state.error != null) {
            return _ErrorView(
              error: state.error!,
              onRetry: () => ref.read(tracesProvider.notifier).refresh(),
            );
          }
          if (state.traces.isEmpty) {
            return const _EmptyView();
          }
          return ListView.builder(
            itemCount: state.traces.length,
            itemBuilder: (context, index) {
              return _TraceRow(trace: state.traces[index]);
            },
          );
        },
      ),
    );
  }
}

/// Expandable row for a single trace summary.
class _TraceRow extends ConsumerStatefulWidget {
  const _TraceRow({required this.trace});

  final TraceSummaryResponse trace;

  @override
  ConsumerState<_TraceRow> createState() => _TraceRowState();
}

class _TraceRowState extends ConsumerState<_TraceRow> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final trace = widget.trace;
    final isError = trace.status == 'error';

    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          InkWell(
            borderRadius: BorderRadius.circular(8),
            onTap: () => setState(() => _expanded = !_expanded),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Row(
                children: [
                  // Status indicator
                  Container(
                    width: 8,
                    height: 8,
                    decoration: BoxDecoration(
                      color: isError ? Colors.red : Colors.green,
                      shape: BoxShape.circle,
                    ),
                  ),
                  const SizedBox(width: 10),

                  // Main info
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          trace.personaId,
                          style: const TextStyle(fontWeight: FontWeight.w600),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          _formatTimestamp(trace.startTime),
                          style: const TextStyle(
                            fontSize: 12,
                            color: Colors.black54,
                          ),
                        ),
                      ],
                    ),
                  ),

                  // Duration
                  Text(
                    _formatDuration(trace.durationMs),
                    style: const TextStyle(
                      fontSize: 13,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  const SizedBox(width: 8),

                  // Expand chevron
                  Icon(
                    _expanded
                        ? Icons.keyboard_arrow_up
                        : Icons.keyboard_arrow_down,
                    size: 20,
                    color: Colors.black45,
                  ),
                ],
              ),
            ),
          ),

          // Expandable span detail
          if (_expanded) _SpanDetail(traceId: trace.traceId),
        ],
      ),
    );
  }

  String _formatTimestamp(DateTime dt) {
    return '${dt.hour.toString().padLeft(2, '0')}:'
        '${dt.minute.toString().padLeft(2, '0')}:'
        '${dt.second.toString().padLeft(2, '0')} '
        '${dt.day}/${dt.month}/${dt.year}';
  }

  String _formatDuration(int ms) {
    if (ms < 1000) return '${ms}ms';
    return '${(ms / 1000).toStringAsFixed(1)}s';
  }
}

/// Span detail widget loaded on demand.
class _SpanDetail extends ConsumerWidget {
  const _SpanDetail({required this.traceId});

  final String traceId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final detailAsync = ref.watch(traceDetailProvider(traceId));

    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 0, 12, 12),
      child: detailAsync.when(
        loading: () => const SizedBox(
          height: 64,
          child: Center(
            child: SizedBox(
              width: 20,
              height: 20,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
          ),
        ),
        error: (e, _) => Text(
          'Failed to load spans: $e',
          style: TextStyle(color: Colors.red.shade700, fontSize: 12),
        ),
        data: (state) {
          if (state.error != null) {
            return Text(
              state.error!,
              style: TextStyle(color: Colors.red.shade700, fontSize: 12),
            );
          }
          final spans = state.detail?.spans.toList() ?? [];
          if (spans.isEmpty) {
            return const Text(
              'No spans recorded',
              style: TextStyle(color: Colors.black45, fontSize: 12),
            );
          }
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Divider(),
              ...spans.map((span) => _SpanRow(span: span)),
            ],
          );
        },
      ),
    );
  }
}

class _SpanRow extends StatelessWidget {
  const _SpanRow({required this.span});

  final SpanEntryResponse span;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        children: [
          const SizedBox(width: 4),
          const Icon(Icons.subdirectory_arrow_right, size: 14, color: Colors.black38),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              span.name,
              style: const TextStyle(fontSize: 12, fontFamily: 'monospace'),
            ),
          ),
          Text(
            '${span.durationMs}ms',
            style: const TextStyle(fontSize: 12, color: Colors.black54),
          ),
        ],
      ),
    );
  }
}

// -- Utility widgets ---------------------------------------------------------

class _EmptyView extends StatelessWidget {
  const _EmptyView();

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.timeline, size: 64, color: Colors.black26),
          SizedBox(height: 12),
          Text(
            'No traces yet',
            style: TextStyle(fontSize: 16, color: Colors.black45),
          ),
          SizedBox(height: 4),
          Text(
            'Send a chat message to generate traces.',
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
