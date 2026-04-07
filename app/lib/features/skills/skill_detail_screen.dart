import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'skill_detail_provider.dart';

/// Read-only detail view for a single skill.
class SkillDetailScreen extends ConsumerWidget {
  const SkillDetailScreen({super.key, required this.skillName});

  final String skillName;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final detailAsync = ref.watch(skillDetailProvider(skillName));

    return Scaffold(
      appBar: AppBar(
        title: Text(detailAsync.value?.skill?.name ?? skillName),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/skills'),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () =>
                ref.read(skillDetailProvider(skillName).notifier).refresh(),
          ),
        ],
      ),
      body: detailAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => _ErrorView(
          error: err.toString(),
          onRetry: () =>
              ref.read(skillDetailProvider(skillName).notifier).refresh(),
        ),
        data: (state) {
          if (state.error != null) {
            return _ErrorView(
              error: state.error!,
              onRetry: () =>
                  ref.read(skillDetailProvider(skillName).notifier).refresh(),
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
