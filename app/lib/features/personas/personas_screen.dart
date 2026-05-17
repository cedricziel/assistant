import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_api/assistant_api.dart';

import '../../shared/platform/widgets.dart';
import 'persona_constants.dart';
import 'personas_provider.dart';

/// Screen that lists all personas.
///
/// Each row shows name, ID, and a default badge.
/// Tapping a row navigates to the persona detail screen.
/// The FAB navigates to the create form.
class PersonasScreen extends ConsumerStatefulWidget {
  const PersonasScreen({super.key});

  @override
  ConsumerState<PersonasScreen> createState() => _PersonasScreenState();
}

class _PersonasScreenState extends ConsumerState<PersonasScreen> {
  final _searchController = TextEditingController();
  String _query = '';

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final personasAsync = ref.watch(personasProvider);

    final fab = FloatingActionButton(
      onPressed: () => context.go('/personas/new'),
      child: const Icon(Icons.add),
    );

    final searchField = Padding(
      padding: const EdgeInsets.fromLTRB(12, 8, 12, 4),
      child: AdaptiveTextField(
        controller: _searchController,
        placeholder: 'Search personas...',
        prefix: const Padding(
          padding: EdgeInsets.symmetric(horizontal: 8),
          child: Icon(Icons.search, size: 20),
        ),
        onChanged: (v) => setState(() => _query = v.toLowerCase()),
      ),
    );

    return AdaptiveScaffold(
      body: CustomScrollView(
        slivers: [
          AdaptiveSliverNavBar(
            title: 'Personas',
            actions: [
              IconButton(
                icon: const Icon(Icons.refresh),
                onPressed: () => ref.read(personasProvider.notifier).refresh(),
              ),
            ],
          ),
          SliverToBoxAdapter(child: searchField),
          personasAsync.when(
            loading: () => const SliverFillRemaining(
              child: Center(child: CircularProgressIndicator.adaptive()),
            ),
            error: (err, _) => SliverFillRemaining(
              child: _ErrorView(
                error: err.toString(),
                onRetry: () => ref.read(personasProvider.notifier).refresh(),
              ),
            ),
            data: (state) {
              if (state.error != null) {
                return SliverFillRemaining(
                  child: _ErrorView(
                    error: state.error!,
                    onRetry: () =>
                        ref.read(personasProvider.notifier).refresh(),
                  ),
                );
              }
              final filtered = _query.isEmpty
                  ? state.personas
                  : state.personas
                        .where(
                          (p) =>
                              p.name.toLowerCase().contains(_query) ||
                              p.id.toLowerCase().contains(_query),
                        )
                        .toList();

              if (filtered.isEmpty) {
                return const SliverFillRemaining(child: _EmptyView());
              }
              return SliverList(
                delegate: SliverChildBuilderDelegate((context, index) {
                  if (index.isOdd) {
                    return const Divider(height: 1, indent: 72);
                  }
                  return _PersonaRow(persona: filtered[index ~/ 2]);
                }, childCount: filtered.length * 2 - 1),
              );
            },
          ),
          const SliverToBoxAdapter(child: SizedBox(height: 80)),
        ],
      ),
      floatingActionButton: fab,
    );
  }
}

class _PersonaRow extends StatelessWidget {
  const _PersonaRow({required this.persona});

  final PersonaSummary persona;

  @override
  Widget build(BuildContext context) {
    final isDefault = persona.id == kDefaultPersonaId;
    return AdaptiveListTile(
      leading: CircleAvatar(
        backgroundColor: isDefault
            ? Theme.of(context).colorScheme.primaryContainer
            : Theme.of(context).colorScheme.surfaceContainerHighest,
        child: Text(
          persona.name.isNotEmpty ? persona.name[0].toUpperCase() : '?',
          style: TextStyle(
            color: isDefault
                ? Theme.of(context).colorScheme.onPrimaryContainer
                : Theme.of(context).colorScheme.onSurface,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
      title: Text(
        persona.name,
        style: const TextStyle(fontWeight: FontWeight.w600),
      ),
      subtitle: Text(
        persona.id,
        style: TextStyle(
          fontSize: 12,
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      ),
      trailing: isDefault
          ? Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
              decoration: BoxDecoration(
                color: Theme.of(context).colorScheme.primaryContainer,
                borderRadius: BorderRadius.circular(12),
              ),
              child: Text(
                'Default',
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: FontWeight.w600,
                  color: Theme.of(context).colorScheme.onPrimaryContainer,
                ),
              ),
            )
          : null,
      onTap: () => context.go('/personas/${persona.id}'),
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
            Icons.person_outline,
            size: 64,
            color: colorScheme.outlineVariant,
          ),
          const SizedBox(height: 12),
          Text(
            'No personas found',
            style: TextStyle(fontSize: 16, color: colorScheme.onSurfaceVariant),
          ),
          const SizedBox(height: 4),
          Text(
            'Create a persona to get started.',
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
          AdaptiveButton.filled(onPressed: onRetry, child: const Text('Retry')),
        ],
      ),
    );
  }
}
