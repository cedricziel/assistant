import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_api/assistant_api.dart';

import '../../shared/platform/platform.dart';
import '../personas/personas_provider.dart';
import 'skills_provider.dart';

/// Read-only skill discovery screen.
///
/// Shows skills available to the active persona with name, description,
/// and enabled/disabled badge.
class SkillsScreen extends ConsumerWidget {
  const SkillsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final skillsAsync = ref.watch(skillsProvider);
    final personasAsync = ref.watch(personasProvider);
    final activePersonaName =
        personasAsync.value?.activePersona?.name ?? 'Active Persona';

    final fab = FloatingActionButton(
      onPressed: () => context.go('/skills/new'),
      tooltip: 'New skill',
      child: const Icon(Icons.add),
    );

    if (isAppleTouch) {
      return Scaffold(
        floatingActionButton: fab,
        body: CustomScrollView(
          slivers: [
            CupertinoSliverNavigationBar(
              largeTitle: Text('Skills — $activePersonaName'),
              trailing: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  IconButton(
                    icon: const Icon(Icons.refresh),
                    onPressed: () =>
                        ref.read(skillsProvider.notifier).refresh(),
                  ),
                ],
              ),
            ),
            skillsAsync.when(
              loading: () => const SliverFillRemaining(
                child: Center(child: CircularProgressIndicator()),
              ),
              error: (err, _) => SliverFillRemaining(
                child: _ErrorView(
                  error: err.toString(),
                  onRetry: () => ref.read(skillsProvider.notifier).refresh(),
                ),
              ),
              data: (state) {
                if (state.error != null) {
                  return SliverFillRemaining(
                    child: _ErrorView(
                      error: state.error!,
                      onRetry: () =>
                          ref.read(skillsProvider.notifier).refresh(),
                    ),
                  );
                }
                if (state.skills.isEmpty) {
                  return SliverFillRemaining(
                    child: _EmptyView(personaName: activePersonaName),
                  );
                }
                return SliverList(
                  delegate: SliverChildBuilderDelegate((context, index) {
                    if (index.isOdd) {
                      return const Divider(height: 1, indent: 72);
                    }
                    return _SkillRow(skill: state.skills[index ~/ 2]);
                  }, childCount: state.skills.length * 2 - 1),
                );
              },
            ),
          ],
        ),
      );
    }

    return Scaffold(
      appBar: AppBar(
        title: Text('Skills — $activePersonaName'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/chat'),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => ref.read(skillsProvider.notifier).refresh(),
          ),
        ],
      ),
      floatingActionButton: fab,
      body: skillsAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () => ref.read(skillsProvider.notifier).refresh(),
        ),
        data: (state) {
          if (state.error != null) {
            return _ErrorView(
              error: state.error!,
              onRetry: () => ref.read(skillsProvider.notifier).refresh(),
            );
          }
          if (state.skills.isEmpty) {
            return _EmptyView(personaName: activePersonaName);
          }
          return ListView.separated(
            padding: const EdgeInsets.symmetric(vertical: 8),
            itemCount: state.skills.length,
            separatorBuilder: (context, index) =>
                const Divider(height: 1, indent: 72),
            itemBuilder: (context, index) {
              return _SkillRow(skill: state.skills[index]);
            },
          );
        },
      ),
    );
  }
}

/// A single skill list row. Tapping expands the description; the detail
/// icon opens the full skill detail screen.
class _SkillRow extends StatefulWidget {
  const _SkillRow({required this.skill});

  final SkillEntryResponse skill;

  @override
  State<_SkillRow> createState() => _SkillRowState();
}

class _SkillRowState extends State<_SkillRow> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final skill = widget.skill;
    return InkWell(
      onTap: () => setState(() => _expanded = !_expanded),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            CircleAvatar(
              backgroundColor: skill.enabled
                  ? Colors.green.shade100
                  : Colors.grey.shade200,
              child: Icon(
                skill.enabled ? Icons.extension : Icons.extension_off_outlined,
                size: 20,
                color: skill.enabled
                    ? Colors.green.shade700
                    : Colors.grey.shade500,
              ),
            ),
            const SizedBox(width: 16),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          skill.name,
                          style: const TextStyle(
                            fontWeight: FontWeight.w600,
                            fontSize: 14,
                          ),
                        ),
                      ),
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 3,
                        ),
                        decoration: BoxDecoration(
                          color: skill.enabled
                              ? Colors.green.shade50
                              : Colors.grey.shade100,
                          borderRadius: BorderRadius.circular(12),
                          border: Border.all(
                            color: skill.enabled
                                ? Colors.green.shade300
                                : Colors.grey.shade400,
                          ),
                        ),
                        child: Text(
                          skill.enabled ? 'Enabled' : 'Disabled',
                          style: TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w600,
                            color: skill.enabled
                                ? Colors.green.shade700
                                : Colors.grey.shade600,
                          ),
                        ),
                      ),
                      const SizedBox(width: 4),
                      IconButton(
                        icon: const Icon(Icons.arrow_forward_ios, size: 14),
                        tooltip: 'View detail',
                        visualDensity: VisualDensity.compact,
                        onPressed: () => context.go('/skills/${skill.name}'),
                      ),
                    ],
                  ),
                  if (skill.description.isNotEmpty) ...[
                    const SizedBox(height: 2),
                    Text(
                      skill.description,
                      maxLines: _expanded ? null : 2,
                      overflow: _expanded ? null : TextOverflow.ellipsis,
                      style: const TextStyle(fontSize: 12),
                    ),
                    if (!_expanded)
                      Text(
                        'Tap to expand',
                        style: TextStyle(
                          fontSize: 11,
                          color: Theme.of(context).colorScheme.onSurfaceVariant,
                          fontStyle: FontStyle.italic,
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
}

// -- Utility widgets ---------------------------------------------------------

class _EmptyView extends StatelessWidget {
  const _EmptyView({required this.personaName});

  final String personaName;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.extension_off_outlined,
              size: 64,
              color: Theme.of(context).colorScheme.outlineVariant,
            ),
            const SizedBox(height: 16),
            Text(
              'No skills configured',
              style: TextStyle(
                fontSize: 18,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
                fontWeight: FontWeight.w500,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              'The "$personaName" persona has no skills configured.',
              textAlign: TextAlign.center,
              style: TextStyle(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ],
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
