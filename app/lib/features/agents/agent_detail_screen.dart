import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'agents_provider.dart';

/// Screen that shows the full detail of a single agent.
class AgentDetailScreen extends ConsumerWidget {
  const AgentDetailScreen({super.key, required this.agentId});

  final String agentId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final detailAsync = ref.watch(agentDetailProvider(agentId));

    return Scaffold(
      appBar: AppBar(
        title: const Text('Agent Detail'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/agents'),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () =>
                ref.read(agentDetailProvider(agentId).notifier).refresh(),
          ),
        ],
      ),
      body: detailAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () =>
              ref.read(agentDetailProvider(agentId).notifier).refresh(),
        ),
        data: (state) {
          if (state.error != null) {
            return _ErrorView(
              error: state.error!,
              onRetry: () =>
                  ref.read(agentDetailProvider(agentId).notifier).refresh(),
            );
          }
          final agent = state.agent;
          if (agent == null) {
            return const Center(child: Text('Agent not found'));
          }
          return _AgentDetailBody(agent: agent);
        },
      ),
    );
  }
}

class _AgentDetailBody extends StatelessWidget {
  const _AgentDetailBody({required this.agent});

  final AgentDetail agent;

  @override
  Widget build(BuildContext context) {
    final card = agent.card;
    // Extract readable info from the JSON card object.
    String name = 'Agent';
    String description = '';
    String version = '';
    String url = '';
    List<Map<String, String>> skills = [];

    try {
      if (card != null) {
        final raw = card.value;
        if (raw is Map) {
          name = raw['name']?.toString() ?? agent.id;
          description = raw['description']?.toString() ?? '';
          version = raw['version']?.toString() ?? '';
          url = raw['url']?.toString() ?? '';
          final rawSkills = raw['skills'];
          if (rawSkills is List) {
            for (final s in rawSkills) {
              if (s is Map) {
                skills.add({
                  'name': s['name']?.toString() ?? '',
                  'description': s['description']?.toString() ?? '',
                });
              }
            }
          }
        }
      } else {
        name = agent.id;
      }
    } catch (_) {
      name = agent.id;
    }

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
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
                        name,
                        style: Theme.of(context).textTheme.headlineSmall,
                      ),
                    ),
                    if (agent.isDefault)
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 3,
                        ),
                        decoration: BoxDecoration(
                          color: Theme.of(context).colorScheme.primaryContainer,
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: Text(
                          'Default',
                          style: TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w600,
                            color: Theme.of(
                              context,
                            ).colorScheme.onPrimaryContainer,
                          ),
                        ),
                      ),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  'ID: ${agent.id}',
                  style: const TextStyle(
                    fontSize: 12,
                    color: Colors.black54,
                    fontFamily: 'monospace',
                  ),
                ),
                if (version.isNotEmpty) ...[
                  const SizedBox(height: 4),
                  Text(
                    'Version: $version',
                    style: const TextStyle(fontSize: 12, color: Colors.black54),
                  ),
                ],
                if (description.isNotEmpty) ...[
                  const SizedBox(height: 8),
                  Text(
                    description,
                    style: const TextStyle(color: Colors.black54),
                  ),
                ],
                if (url.isNotEmpty) ...[
                  const SizedBox(height: 8),
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          url,
                          style: const TextStyle(
                            fontSize: 12,
                            fontFamily: 'monospace',
                            color: Colors.black54,
                          ),
                        ),
                      ),
                      IconButton(
                        icon: const Icon(Icons.copy, size: 16),
                        tooltip: 'Copy URL',
                        onPressed: () {
                          Clipboard.setData(ClipboardData(text: url));
                        },
                      ),
                    ],
                  ),
                ],
              ],
            ),
          ),
        ),

        if (skills.isNotEmpty) ...[
          const SizedBox(height: 16),
          Text(
            'Skills (${skills.length})',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          ...skills.map(
            (s) => Card(
              margin: const EdgeInsets.only(bottom: 4),
              child: ListTile(
                leading: const Icon(Icons.extension_outlined, size: 20),
                title: Text(
                  s['name']!,
                  style: const TextStyle(
                    fontWeight: FontWeight.w600,
                    fontSize: 14,
                  ),
                ),
                subtitle: s['description']!.isNotEmpty
                    ? Text(
                        s['description']!,
                        style: const TextStyle(fontSize: 12),
                      )
                    : null,
              ),
            ),
          ),
        ],
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
