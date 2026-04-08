import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'skill_detail_provider.dart';
import 'skills_provider.dart';

/// Detail view for a single skill. Allows editing and deleting user-created skills.
class SkillDetailScreen extends ConsumerStatefulWidget {
  const SkillDetailScreen({super.key, required this.skillName});

  final String skillName;

  @override
  ConsumerState<SkillDetailScreen> createState() => _SkillDetailScreenState();
}

class _SkillDetailScreenState extends ConsumerState<SkillDetailScreen> {
  Future<void> _confirmDelete() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Delete skill?'),
        content: Text(
          'This will permanently delete "${widget.skillName}". This action cannot be undone.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(
              backgroundColor: Colors.red.shade600,
            ),
            onPressed: () => Navigator.of(ctx).pop(true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;

    final error = await ref
        .read(skillsProvider.notifier)
        .deleteSkill(widget.skillName);
    if (!mounted) return;
    if (error != null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Delete failed: $error'),
          backgroundColor: Colors.red.shade600,
          behavior: SnackBarBehavior.floating,
        ),
      );
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('Skill deleted'),
        behavior: SnackBarBehavior.floating,
      ),
    );
    context.go('/skills');
  }

  @override
  Widget build(BuildContext context) {
    final detailAsync = ref.watch(skillDetailProvider(widget.skillName));

    return Scaffold(
      appBar: AppBar(
        title: Text(detailAsync.value?.skill?.name ?? widget.skillName),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/skills'),
        ),
        actions: [
          if (detailAsync.value?.skill != null &&
              detailAsync.value!.skill!.isBuiltin == false) ...[
            IconButton(
              icon: const Icon(Icons.edit_outlined),
              tooltip: 'Edit skill',
              onPressed: () => context.go('/skills/${widget.skillName}/edit'),
            ),
            IconButton(
              icon: Icon(Icons.delete_outline, color: Colors.red.shade400),
              tooltip: 'Delete skill',
              onPressed: _confirmDelete,
            ),
          ],
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () =>
                ref.read(skillDetailProvider(widget.skillName).notifier).refresh(),
          ),
        ],
      ),
      body: detailAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () =>
              ref.read(skillDetailProvider(widget.skillName).notifier).refresh(),
        ),
        data: (state) {
          if (state.error != null) {
            return _ErrorView(
              error: state.error!,
              onRetry: () =>
                  ref.read(skillDetailProvider(widget.skillName).notifier).refresh(),
            );
          }
          final skill = state.skill;
          if (skill == null) {
            return const Center(child: Text('Skill not found'));
          }
          return _SkillDetailBody(skill: skill);
        },
      ),
    );
  }
}

class _SkillDetailBody extends StatefulWidget {
  const _SkillDetailBody({required this.skill});

  final dynamic skill;

  @override
  State<_SkillDetailBody> createState() => _SkillDetailBodyState();
}

class _SkillDetailBodyState extends State<_SkillDetailBody> {
  bool _copied = false;
  Timer? _resetTimer;

  @override
  void dispose() {
    _resetTimer?.cancel();
    super.dispose();
  }

  void _copyBody() {
    Clipboard.setData(ClipboardData(text: widget.skill.body));
    setState(() => _copied = true);
    _resetTimer?.cancel();
    _resetTimer = Timer(const Duration(seconds: 2), () {
      if (mounted) setState(() => _copied = false);
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final skill = widget.skill;
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // -- Header chip row ------------------------------------------------
          Row(
            children: [
              _Badge(
                label: skill.isBuiltin ? 'Built-in' : 'User',
                color: skill.isBuiltin
                    ? theme.colorScheme.primaryContainer
                    : theme.colorScheme.tertiaryContainer,
              ),
              const SizedBox(width: 8),
              _Badge(
                label: skill.source_,
                color: theme.colorScheme.surfaceContainerHighest,
              ),
            ],
          ),
          const SizedBox(height: 16),

          // -- Description ----------------------------------------------------
          if (skill.description.isNotEmpty) ...[
            Text(
              'Description',
              style: theme.textTheme.labelLarge
                  ?.copyWith(color: theme.colorScheme.primary),
            ),
            const SizedBox(height: 4),
            Text(skill.description, style: theme.textTheme.bodyMedium),
            const SizedBox(height: 20),
          ],

          // -- Body -----------------------------------------------------------
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                'Skill Body',
                style: theme.textTheme.labelLarge
                    ?.copyWith(color: theme.colorScheme.primary),
              ),
              IconButton(
                icon: Icon(
                  _copied ? Icons.check : Icons.copy,
                  size: 16,
                  color: _copied ? Colors.green.shade600 : null,
                ),
                tooltip: _copied ? 'Copied!' : 'Copy body',
                onPressed: _copyBody,
              ),
            ],
          ),
          const SizedBox(height: 4),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: SelectableText(
              skill.body,
              style: const TextStyle(
                fontFamily: 'monospace',
                fontSize: 12,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _Badge extends StatelessWidget {
  const _Badge({required this.label, required this.color});

  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Text(
        label,
        style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w600),
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
