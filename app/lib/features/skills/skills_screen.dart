import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../api/models/skill.dart';
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
        personasAsync.valueOrNull?.activePersona?.name ?? 'Active Persona';

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

/// A single skill list row.
class _SkillRow extends StatelessWidget {
  const _SkillRow({required this.skill});

  final Skill skill;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: CircleAvatar(
        backgroundColor:
            skill.enabled ? Colors.green.shade100 : Colors.grey.shade200,
        child: Icon(
          skill.enabled ? Icons.extension : Icons.extension_off_outlined,
          size: 20,
          color: skill.enabled ? Colors.green.shade700 : Colors.grey.shade500,
        ),
      ),
      title: Text(
        skill.name,
        style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 14),
      ),
      subtitle: skill.description.isNotEmpty
          ? Text(
              skill.description,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontSize: 12),
            )
          : null,
      trailing: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
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
            const Icon(
              Icons.extension_off_outlined,
              size: 64,
              color: Colors.black26,
            ),
            const SizedBox(height: 16),
            Text(
              'No skills configured',
              style: const TextStyle(
                fontSize: 18,
                color: Colors.black45,
                fontWeight: FontWeight.w500,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              'The "$personaName" persona has no skills configured.',
              textAlign: TextAlign.center,
              style: const TextStyle(color: Colors.black38),
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
