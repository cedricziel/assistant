import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_api/assistant_api.dart';

import '../../shared/platform/platform.dart';
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

    if (isAppleTouch) {
      return Scaffold(
        body: CustomScrollView(
          slivers: [
            CupertinoSliverNavigationBar(
              largeTitle: const Text('Traces'),
              trailing: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  IconButton(
                    icon: const Icon(Icons.refresh),
                    onPressed: () =>
                        ref.read(tracesProvider.notifier).refresh(),
                  ),
                ],
              ),
            ),
            tracesAsync.when(
              loading: () => const SliverFillRemaining(
                child: Center(child: CircularProgressIndicator()),
              ),
              error: (err, _) => SliverFillRemaining(
                child: _ErrorView(
                  error: err.toString(),
                  onRetry: () => ref.read(tracesProvider.notifier).refresh(),
                ),
              ),
              data: (state) {
                if (state.error != null) {
                  return SliverFillRemaining(
                    child: _ErrorView(
                      error: state.error!,
                      onRetry: () =>
                          ref.read(tracesProvider.notifier).refresh(),
                    ),
                  );
                }
                if (state.traces.isEmpty) {
                  return const SliverFillRemaining(child: _EmptyView());
                }
                return SliverList(
                  delegate: SliverChildBuilderDelegate((context, index) {
                    return _TraceRow(trace: state.traces[index]);
                  }, childCount: state.traces.length),
                );
              },
            ),
          ],
        ),
      );
    }

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
              final trace = state.traces[index];
              return _TraceRow(trace: trace);
            },
          );
        },
      ),
    );
  }
}

/// Tappable row for a single trace summary — navigates to full detail on tap.
class _TraceRow extends StatelessWidget {
  const _TraceRow({required this.trace});

  final TraceSummaryResponse trace;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isError = trace.status == 'error';

    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: () => context.go('/traces/${trace.traceId}'),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            children: [
              // Status indicator dot
              Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  color: isError ? colorScheme.error : Colors.green,
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
                      style: TextStyle(
                        fontSize: 12,
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                    if (trace.skillName != null)
                      Text(
                        trace.skillName!,
                        style: TextStyle(
                          fontSize: 11,
                          color: colorScheme.onSurfaceVariant,
                          fontFamily: 'monospace',
                        ),
                      ),
                  ],
                ),
              ),

              // Duration + chevron
              Column(
                crossAxisAlignment: CrossAxisAlignment.end,
                children: [
                  Text(
                    _formatDuration(trace.durationMs),
                    style: const TextStyle(
                      fontSize: 13,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Icon(
                    Icons.arrow_forward_ios,
                    size: 12,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ],
              ),
            ],
          ),
        ),
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

// -- Utility widgets ---------------------------------------------------------

class _EmptyView extends StatelessWidget {
  const _EmptyView();

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.timeline, size: 64, color: colorScheme.outlineVariant),
          const SizedBox(height: 12),
          Text(
            'No traces yet',
            style: TextStyle(fontSize: 16, color: colorScheme.onSurfaceVariant),
          ),
          const SizedBox(height: 4),
          Text(
            'Send a chat message to generate traces.',
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
    final colorScheme = Theme.of(context).colorScheme;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.error_outline, color: colorScheme.error, size: 48),
          const SizedBox(height: 12),
          Text(error, textAlign: TextAlign.center),
          const SizedBox(height: 12),
          FilledButton(onPressed: onRetry, child: const Text('Retry')),
        ],
      ),
    );
  }
}
