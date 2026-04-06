import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_api/assistant_api.dart';

import 'persona_detail_provider.dart';

/// Screen that shows the full detail of a single persona.
class PersonaDetailScreen extends ConsumerWidget {
  const PersonaDetailScreen({super.key, required this.personaId});

  final String personaId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final detailAsync = ref.watch(personaDetailProvider(personaId));

    return Scaffold(
      appBar: AppBar(
        title: Text(
          detailAsync.value?.detail?.name ?? 'Persona',
        ),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/contexts'),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () =>
                ref.read(personaDetailProvider(personaId).notifier).refresh(),
          ),
        ],
      ),
      body: detailAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () =>
              ref.read(personaDetailProvider(personaId).notifier).refresh(),
        ),
        data: (state) {
          if (state.error != null) {
            return _ErrorView(
              error: state.error!,
              onRetry: () =>
                  ref.read(personaDetailProvider(personaId).notifier).refresh(),
            );
          }
          final detail = state.detail;
          if (detail == null) {
            return const Center(child: Text('Persona not found'));
          }
          return _PersonaDetailBody(detail: detail);
        },
      ),
    );
  }
}

class _PersonaDetailBody extends StatelessWidget {
  const _PersonaDetailBody({required this.detail});

  final PersonaDetail detail;

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
                    if (detail.isDefault)
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 3,
                        ),
                        decoration: BoxDecoration(
                          color: Theme.of(context)
                              .colorScheme
                              .primaryContainer,
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: Text(
                          'Default',
                          style: TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w600,
                            color: Theme.of(context)
                                .colorScheme
                                .onPrimaryContainer,
                          ),
                        ),
                      ),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  'ID: ${detail.id}',
                  style: const TextStyle(
                    fontSize: 12,
                    color: Colors.black54,
                    fontFamily: 'monospace',
                  ),
                ),
                const SizedBox(height: 8),
                _SkillAccessChip(mode: detail.skillAccessMode),
              ],
            ),
          ),
        ),

        const SizedBox(height: 16),

        // File slots.
        Text(
          'File Slots',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        ...detail.files.map(
          (slot) => _FileSlotTile(
            slot: slot,
            personaId: detail.id,
          ),
        ),
      ],
    );
  }
}

class _SkillAccessChip extends StatelessWidget {
  const _SkillAccessChip({required this.mode});

  final String mode;

  @override
  Widget build(BuildContext context) {
    final color = switch (mode) {
      'all' => Colors.green,
      'whitelist' => Colors.blue,
      'blacklist' => Colors.orange,
      _ => Colors.grey,
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.shade50,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.shade300),
      ),
      child: Text(
        'Skills: $mode',
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          color: color.shade700,
        ),
      ),
    );
  }
}

class _FileSlotTile extends StatelessWidget {
  const _FileSlotTile({required this.slot, required this.personaId});

  final PersonaFileSlot slot;
  final String personaId;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 4),
      child: ListTile(
        leading: Icon(
          slot.exists ? Icons.description : Icons.description_outlined,
          color: slot.exists ? Colors.green.shade600 : Colors.black38,
        ),
        title: Text(
          slot.filename,
          style: const TextStyle(fontFamily: 'monospace', fontSize: 14),
        ),
        subtitle: Text(
          slot.description,
          style: const TextStyle(fontSize: 12),
        ),
        trailing: Container(
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
          decoration: BoxDecoration(
            color: slot.exists
                ? Colors.green.shade50
                : Colors.grey.shade100,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: slot.exists
                  ? Colors.green.shade300
                  : Colors.grey.shade400,
            ),
          ),
          child: Text(
            slot.exists ? 'Exists' : 'Missing',
            style: TextStyle(
              fontSize: 10,
              fontWeight: FontWeight.w600,
              color: slot.exists
                  ? Colors.green.shade700
                  : Colors.grey.shade600,
            ),
          ),
        ),
        onTap: () => context.go(
          '/contexts/$personaId/files/${slot.filename}',
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
