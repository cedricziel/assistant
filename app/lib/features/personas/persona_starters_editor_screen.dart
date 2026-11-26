import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import 'persona_starters_controller.dart';

/// Editor for a Persona's starter prompts: add (up to 4), edit, reorder and
/// remove. Validates the API limits before saving.
class PersonaStartersEditorScreen extends ConsumerStatefulWidget {
  const PersonaStartersEditorScreen({super.key, required this.personaId});

  final String personaId;

  @override
  ConsumerState<PersonaStartersEditorScreen> createState() =>
      _PersonaStartersEditorScreenState();
}

class _PersonaStartersEditorScreenState
    extends ConsumerState<PersonaStartersEditorScreen> {
  final List<TextEditingController> _fields = [];
  bool _initialized = false;
  bool _saving = false;
  String? _saveError;

  String get _backLocation => '/personas/${widget.personaId}';

  @override
  void dispose() {
    for (final field in _fields) {
      field.dispose();
    }
    super.dispose();
  }

  List<String> get _prompts => [for (final f in _fields) f.text];

  void _initFrom(List<String> prompts) {
    if (_initialized) return;
    _initialized = true;
    _fields.addAll(prompts.map((p) => TextEditingController(text: p)));
  }

  void _add() => setState(() => _fields.add(TextEditingController()));

  void _remove(int index) {
    setState(() => _fields.removeAt(index).dispose());
  }

  void _move(int index, int delta) {
    setState(() => _fields.insert(index + delta, _fields.removeAt(index)));
  }

  Future<void> _save() async {
    setState(() {
      _saving = true;
      _saveError = null;
    });
    try {
      await ref
          .read(personaStartersControllerProvider(widget.personaId))
          .save(_prompts);
      if (mounted) context.go(_backLocation);
    } catch (e) {
      if (mounted) {
        setState(() {
          _saving = false;
          _saveError = e.toString();
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final view = ref.watch(personaStartersProvider(widget.personaId));
    final validation = validateStarterPrompts(_prompts);
    final canSave = _initialized && validation == null && !_saving;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Starter prompts'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go(_backLocation),
        ),
        actions: [
          TextButton(
            key: const Key('starters_save'),
            onPressed: canSave ? _save : null,
            child: const Text('Save'),
          ),
        ],
      ),
      body: view.when(
        loading: () =>
            const Center(child: CircularProgressIndicator.adaptive()),
        error: (err, _) => Center(child: Text(err.toString())),
        data: (data) {
          if (!data.canEdit) {
            return const Center(
              child: Text(
                'You do not have permission to edit starter prompts.',
              ),
            );
          }
          _initFrom(data.prompts);
          return _buildEditor(context, validation);
        },
      ),
    );
  }

  Widget _buildEditor(BuildContext context, String? validation) {
    final theme = Theme.of(context);
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(
          'Shown as suggestions in an empty chat. Up to $kMaxStarterPrompts '
          'prompts, $kMaxStarterPromptLength characters each.',
          style: theme.textTheme.bodySmall,
        ),
        const SizedBox(height: 12),
        for (var i = 0; i < _fields.length; i++)
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: TextField(
                    key: Key('starter_field_$i'),
                    controller: _fields[i],
                    minLines: 1,
                    maxLines: 3,
                    decoration: InputDecoration(
                      labelText: 'Prompt ${i + 1}',
                      border: const OutlineInputBorder(),
                    ),
                    onChanged: (_) => setState(() {}),
                  ),
                ),
                IconButton(
                  key: Key('starter_up_$i'),
                  tooltip: 'Move up',
                  icon: const Icon(Icons.arrow_upward),
                  onPressed: i == 0 ? null : () => _move(i, -1),
                ),
                IconButton(
                  key: Key('starter_down_$i'),
                  tooltip: 'Move down',
                  icon: const Icon(Icons.arrow_downward),
                  onPressed: i == _fields.length - 1 ? null : () => _move(i, 1),
                ),
                IconButton(
                  key: Key('starter_remove_$i'),
                  tooltip: 'Remove',
                  icon: const Icon(Icons.delete_outline),
                  onPressed: () => _remove(i),
                ),
              ],
            ),
          ),
        Align(
          alignment: Alignment.centerLeft,
          child: TextButton.icon(
            key: const Key('starter_add'),
            onPressed: _fields.length >= kMaxStarterPrompts ? null : _add,
            icon: const Icon(Icons.add),
            label: const Text('Add prompt'),
          ),
        ),
        if (validation != null)
          Text(validation, style: TextStyle(color: theme.colorScheme.error)),
        if (_saveError != null)
          Text(_saveError!, style: TextStyle(color: theme.colorScheme.error)),
      ],
    );
  }
}
