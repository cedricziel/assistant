import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'skill_detail_provider.dart';
import 'skills_provider.dart';

/// Screen for creating a new skill or editing an existing one.
///
/// Pass [skillName] to enter edit mode; omit (null) for create mode.
class SkillCreateEditScreen extends ConsumerStatefulWidget {
  const SkillCreateEditScreen({super.key, this.skillName});

  final String? skillName;

  bool get isEdit => skillName != null;

  @override
  ConsumerState<SkillCreateEditScreen> createState() =>
      _SkillCreateEditScreenState();
}

class _SkillCreateEditScreenState extends ConsumerState<SkillCreateEditScreen> {
  final _formKey = GlobalKey<FormState>();
  final _nameController = TextEditingController();
  final _descController = TextEditingController();
  final _bodyController = TextEditingController();

  bool _submitting = false;
  String? _error;
  bool _loaded = false;

  @override
  void dispose() {
    _nameController.dispose();
    _descController.dispose();
    _bodyController.dispose();
    super.dispose();
  }

  /// Pre-populate fields when editing an existing skill.
  void _populateFromState(SkillDetailState state) {
    if (_loaded) return;
    final skill = state.skill;
    if (skill == null) return;
    _loaded = true;
    _nameController.text = skill.name;
    _descController.text = skill.description;
    _bodyController.text = skill.body;
  }

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) return;
    setState(() {
      _submitting = true;
      _error = null;
    });

    final notifier = ref.read(skillsProvider.notifier);
    String? error;

    if (widget.isEdit) {
      error = await notifier.updateSkill(
        name: widget.skillName!,
        description: _descController.text.trim(),
        body: _bodyController.text,
      );
    } else {
      error = await notifier.createSkill(
        name: _nameController.text.trim(),
        description: _descController.text.trim(),
        body: _bodyController.text,
      );
    }

    if (!mounted) return;
    if (error != null) {
      setState(() {
        _submitting = false;
        _error = error;
      });
      return;
    }

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(widget.isEdit ? 'Skill updated' : 'Skill created'),
        behavior: SnackBarBehavior.floating,
      ),
    );
    context.go('/skills');
  }

  @override
  Widget build(BuildContext context) {
    // In edit mode, watch the detail to pre-populate fields.
    if (widget.isEdit) {
      final detailAsync = ref.watch(skillDetailProvider(widget.skillName!));
      detailAsync.whenData(_populateFromState);
    }

    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: Text(widget.isEdit ? 'Edit Skill' : 'New Skill'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => widget.isEdit
              ? context.go('/skills/${widget.skillName}')
              : context.go('/skills'),
        ),
        actions: [
          if (_submitting)
            const Padding(
              padding: EdgeInsets.symmetric(horizontal: 16),
              child: SizedBox(
                width: 20,
                height: 20,
                child: CircularProgressIndicator.adaptive(strokeWidth: 2),
              ),
            )
          else
            TextButton(
              onPressed: _submit,
              child: Text(
                widget.isEdit ? 'Save' : 'Create',
                style: TextStyle(
                  fontWeight: FontWeight.w600,
                  color: theme.colorScheme.primary,
                ),
              ),
            ),
        ],
      ),
      body: Form(
        key: _formKey,
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            if (_error != null) ...[
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: theme.colorScheme.errorContainer,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: theme.colorScheme.error),
                ),
                child: Text(
                  _error!,
                  style: TextStyle(
                    color: theme.colorScheme.onErrorContainer,
                    fontSize: 13,
                  ),
                ),
              ),
              const SizedBox(height: 16),
            ],

            // Name (create-only)
            TextFormField(
              controller: _nameController,
              enabled: !widget.isEdit && !_submitting,
              decoration: InputDecoration(
                labelText: 'Skill name',
                hintText: 'e.g. my-custom-skill',
                helperText:
                    'Lowercase letters, digits, hyphens and underscores',
                border: const OutlineInputBorder(),
                filled: widget.isEdit,
                fillColor: widget.isEdit ? Colors.grey.shade100 : null,
              ),
              style: const TextStyle(fontFamily: 'monospace'),
              validator: (v) {
                if (widget.isEdit) return null;
                if (v == null || v.trim().isEmpty) return 'Name is required';
                if (!RegExp(r'^[a-z0-9_-]+$').hasMatch(v.trim())) {
                  return 'Only lowercase letters, digits, hyphens and underscores';
                }
                return null;
              },
            ),
            const SizedBox(height: 16),

            // Description
            TextFormField(
              controller: _descController,
              enabled: !_submitting,
              maxLines: 3,
              decoration: const InputDecoration(
                labelText: 'Description',
                hintText: 'Briefly describe what this skill does',
                border: OutlineInputBorder(),
                alignLabelWithHint: true,
              ),
              validator: (v) => (v == null || v.trim().isEmpty)
                  ? 'Description is required'
                  : null,
            ),
            const SizedBox(height: 16),

            // Body (markdown/instruction content)
            TextFormField(
              controller: _bodyController,
              enabled: !_submitting,
              maxLines: null,
              minLines: 12,
              decoration: const InputDecoration(
                labelText: 'Skill body',
                hintText:
                    '# SKILL.md\nWrite the skill instructions in Markdown...',
                border: OutlineInputBorder(),
                alignLabelWithHint: true,
              ),
              style: const TextStyle(fontFamily: 'monospace', fontSize: 13),
              validator: (v) =>
                  (v == null || v.isEmpty) ? 'Body is required' : null,
            ),

            const SizedBox(height: 24),

            FilledButton(
              onPressed: _submitting ? null : _submit,
              child: Text(widget.isEdit ? 'Save Changes' : 'Create Skill'),
            ),
          ],
        ),
      ),
    );
  }
}
