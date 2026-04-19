import 'dart:async';

import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_api/assistant_api.dart';

import '../../shared/platform/platform.dart';
import 'logs_provider.dart';

/// Observability screen that lists recent structured log entries.
///
/// - Shows timestamp, severity chip, target module, and message.
/// - Keyword filter text field at the top with debouncing.
class LogsScreen extends ConsumerStatefulWidget {
  const LogsScreen({super.key});

  @override
  ConsumerState<LogsScreen> createState() => _LogsScreenState();
}

class _LogsScreenState extends ConsumerState<LogsScreen> {
  final _searchController = TextEditingController();
  Timer? _debounce;
  bool _searching = false;

  @override
  void dispose() {
    _searchController.dispose();
    _debounce?.cancel();
    super.dispose();
  }

  void _onSearchChanged(String query) {
    setState(() => _searching = true);
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 400), () {
      ref.read(logsProvider.notifier).setSearch(query);
      if (mounted) setState(() => _searching = false);
    });
  }

  @override
  Widget build(BuildContext context) {
    final logsAsync = ref.watch(logsProvider);

    final searchField = Padding(
      padding: const EdgeInsets.fromLTRB(12, 8, 12, 4),
      child: TextField(
        key: const Key('log_search_field'),
        controller: _searchController,
        decoration: InputDecoration(
          hintText: 'Filter by keyword...',
          prefixIcon: const Icon(Icons.search, size: 20),
          suffixIcon: _searching
              ? const Padding(
                  padding: EdgeInsets.all(12),
                  child: SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
                )
              : null,
          border: const OutlineInputBorder(),
          contentPadding: const EdgeInsets.symmetric(vertical: 8),
          isDense: true,
        ),
        onChanged: _onSearchChanged,
      ),
    );

    if (isAppleTouch) {
      return Scaffold(
        body: GestureDetector(
          onTap: () => FocusScope.of(context).unfocus(),
          child: CustomScrollView(
            slivers: [
              CupertinoSliverNavigationBar(
                largeTitle: const Text('Logs'),
                trailing: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    IconButton(
                      icon: const Icon(Icons.refresh),
                      onPressed: () =>
                          ref.read(logsProvider.notifier).refresh(),
                    ),
                  ],
                ),
              ),
              SliverToBoxAdapter(child: searchField),
              logsAsync.when(
                loading: () => const SliverFillRemaining(
                  child: Center(child: CircularProgressIndicator()),
                ),
                error: (err, _) => SliverFillRemaining(
                  child: _ErrorView(
                    error: err.toString(),
                    onRetry: () => ref.read(logsProvider.notifier).refresh(),
                  ),
                ),
                data: (state) {
                  if (state.error != null) {
                    return SliverFillRemaining(
                      child: _ErrorView(
                        error: state.error!,
                        onRetry: () =>
                            ref.read(logsProvider.notifier).refresh(),
                      ),
                    );
                  }
                  if (state.logs.isEmpty) {
                    return SliverFillRemaining(
                      child: Center(
                        child: state.searchQuery.isNotEmpty
                            ? Text(
                                'No logs matching "${state.searchQuery}"',
                                style: TextStyle(
                                  color: Theme.of(
                                    context,
                                  ).colorScheme.onSurfaceVariant,
                                ),
                              )
                            : const _EmptyView(),
                      ),
                    );
                  }
                  return SliverList(
                    delegate: SliverChildBuilderDelegate((context, index) {
                      if (index.isOdd) {
                        return const Divider(
                          height: 1,
                          indent: 12,
                          endIndent: 12,
                        );
                      }
                      return _LogRow(entry: state.logs[index ~/ 2]);
                    }, childCount: state.logs.length * 2 - 1),
                  );
                },
              ),
            ],
          ),
        ),
      );
    }

    return Scaffold(
      appBar: AppBar(
        title: const Text('Logs'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/chat'),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => ref.read(logsProvider.notifier).refresh(),
          ),
        ],
      ),
      body: GestureDetector(
        onTap: () => FocusScope.of(context).unfocus(),
        child: Column(
          children: [
            searchField,
            // Log list
            Expanded(
              child: logsAsync.when(
                loading: () => const Center(child: CircularProgressIndicator()),
                error: (err, _) => _ErrorView(
                  error: err.toString(),
                  onRetry: () => ref.read(logsProvider.notifier).refresh(),
                ),
                data: (state) {
                  if (state.error != null) {
                    return _ErrorView(
                      error: state.error!,
                      onRetry: () => ref.read(logsProvider.notifier).refresh(),
                    );
                  }
                  if (state.logs.isEmpty) {
                    return Center(
                      child: state.searchQuery.isNotEmpty
                          ? Text(
                              'No logs matching "${state.searchQuery}"',
                              style: TextStyle(
                                color: Theme.of(
                                  context,
                                ).colorScheme.onSurfaceVariant,
                              ),
                            )
                          : const _EmptyView(),
                    );
                  }
                  return ListView.separated(
                    itemCount: state.logs.length,
                    separatorBuilder: (context, index) =>
                        const Divider(height: 1, indent: 12, endIndent: 12),
                    itemBuilder: (context, index) {
                      return _LogRow(entry: state.logs[index]);
                    },
                  );
                },
              ),
            ),
          ],
        ),
      ), // GestureDetector
    );
  }
}

/// A single log entry row. Tapping expands long messages.
class _LogRow extends StatefulWidget {
  const _LogRow({required this.entry});

  final LogEntryResponse entry;

  @override
  State<_LogRow> createState() => _LogRowState();
}

class _LogRowState extends State<_LogRow> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final entry = widget.entry;
    return InkWell(
      onTap: () => setState(() => _expanded = !_expanded),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Timestamp
            SizedBox(
              width: 68,
              child: Text(
                _formatTime(entry.timestamp),
                style: TextStyle(
                  fontSize: 11,
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                  fontFamily: 'monospace',
                ),
              ),
            ),

            const SizedBox(width: 8),

            // Severity chip
            _SeverityChip(severity: entry.severity),

            const SizedBox(width: 8),

            // Message + target
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    entry.message,
                    style: const TextStyle(fontSize: 13),
                    maxLines: _expanded ? null : 2,
                    overflow: _expanded ? null : TextOverflow.ellipsis,
                  ),
                  if (entry.target.isNotEmpty) ...[
                    const SizedBox(height: 2),
                    Text(
                      entry.target,
                      style: TextStyle(
                        fontSize: 11,
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                        fontFamily: 'monospace',
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  String _formatTime(DateTime dt) {
    return '${dt.hour.toString().padLeft(2, '0')}:'
        '${dt.minute.toString().padLeft(2, '0')}:'
        '${dt.second.toString().padLeft(2, '0')}';
  }
}

/// Colour-coded severity chip.
class _SeverityChip extends StatelessWidget {
  const _SeverityChip({required this.severity});

  final String severity;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final (bgColor, textColor) = switch (severity.toUpperCase()) {
      'ERROR' => (colorScheme.errorContainer, colorScheme.error),
      'WARN' => (Colors.orange.shade100, Colors.orange.shade800),
      'INFO' => (Colors.blue.shade100, Colors.blue.shade800),
      'DEBUG' => (Colors.grey.shade200, Colors.grey.shade700),
      _ => (Colors.purple.shade50, Colors.purple.shade700),
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: bgColor,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        severity.length > 4 ? severity.substring(0, 4) : severity,
        style: TextStyle(
          fontSize: 10,
          fontWeight: FontWeight.bold,
          color: textColor,
          letterSpacing: 0.5,
        ),
      ),
    );
  }
}

// -- Utility widgets ---------------------------------------------------------

class _EmptyView extends StatelessWidget {
  const _EmptyView();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.article_outlined,
            size: 64,
            color: Theme.of(context).colorScheme.outlineVariant,
          ),
          const SizedBox(height: 12),
          Text(
            'No log entries',
            style: TextStyle(
              fontSize: 16,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
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
