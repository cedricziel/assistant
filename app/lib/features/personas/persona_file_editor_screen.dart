import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../api/api_client.dart';
import '../connection/connection_provider.dart';

/// Provider for the file content of a given persona + filename pair.
final personaFileContentProvider = FutureProvider.autoDispose
    .family<PersonaFileContent?, ({String personaId, String filename})>(
  (ref, args) async {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    final api = ApiClient(baseUrl: profile.baseUrl, token: profile.token);
    try {
      final response = await api.personas.getPersonaFile(
        id: args.personaId,
        filename: args.filename,
      );
      return response.data;
    } catch (_) {
      return null;
    }
  },
);

/// Screen that displays a multi-line text editor for a persona file.
///
/// Loads the file via `GET /api/personas/:id/files/:filename` and saves via
/// `PUT /api/personas/:id/files/:filename`.
class PersonaFileEditorScreen extends ConsumerStatefulWidget {
  const PersonaFileEditorScreen({
    super.key,
    required this.personaId,
    required this.filename,
  });

  final String personaId;
  final String filename;

  @override
  ConsumerState<PersonaFileEditorScreen> createState() =>
      _PersonaFileEditorScreenState();
}

class _PersonaFileEditorScreenState
    extends ConsumerState<PersonaFileEditorScreen> {
  final _controller = TextEditingController();
  bool _dirty = false;
  bool _saving = false;
  String? _saveError;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return;
    setState(() {
      _saving = true;
      _saveError = null;
    });

    try {
      final api = ApiClient(baseUrl: profile.baseUrl, token: profile.token);
      await api.personas.putPersonaFile(
        id: widget.personaId,
        filename: widget.filename,
        writePersonaFileRequest: WritePersonaFileRequest(
          (b) => b.content = _controller.text,
        ),
      );
      if (mounted) {
        setState(() {
          _dirty = false;
          _saving = false;
        });
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Saved')),
        );
      }
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
    final contentAsync = ref.watch(
      personaFileContentProvider(
        (personaId: widget.personaId, filename: widget.filename),
      ),
    );

    Future<bool> confirmDiscard() async {
      final confirm = await showDialog<bool>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: const Text('Discard changes?'),
          content: const Text('You have unsaved changes. Leave without saving?'),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(false),
              child: const Text('Keep editing'),
            ),
            FilledButton(
              onPressed: () => Navigator.of(ctx).pop(true),
              child: const Text('Discard'),
            ),
          ],
        ),
      );
      return confirm ?? false;
    }

    return PopScope(
      canPop: !_dirty,
      onPopInvokedWithResult: (didPop, _) async {
        if (didPop) return;
        final leave = await confirmDiscard();
        if (leave && context.mounted) {
          context.go('/contexts/${widget.personaId}');
        }
      },
      child: Scaffold(
      appBar: AppBar(
        title: Text(widget.filename),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () async {
            if (_dirty) {
              final leave = await confirmDiscard();
              if (leave && context.mounted) {
                context.go('/contexts/${widget.personaId}');
              }
            } else {
              context.go('/contexts/${widget.personaId}');
            }
          },
        ),
        actions: [
          if (_dirty)
            TextButton(
              onPressed: _saving ? null : _save,
              child: _saving
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Text('Save'),
            ),
        ],
      ),
      body: SafeArea(
        child: contentAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, _) => Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.error_outline, color: Colors.red.shade600, size: 48),
              const SizedBox(height: 12),
              Text(err.toString(), textAlign: TextAlign.center),
            ],
          ),
        ),
        data: (content) {
          // Initialise the controller once.
          if (!_dirty && content != null) {
            final fileContent = content.content;
            WidgetsBinding.instance.addPostFrameCallback((_) {
              if (mounted && _controller.text != fileContent) {
                _controller.text = fileContent;
              }
            });
          }

          return Column(
            children: [
              if (_saveError != null)
                Material(
                  color: Colors.red.shade50,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 8,
                    ),
                    child: Row(
                      children: [
                        Icon(
                          Icons.warning_amber_outlined,
                          color: Colors.red.shade700,
                          size: 18,
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            _saveError!,
                            style: TextStyle(color: Colors.red.shade700),
                          ),
                        ),
                        IconButton(
                          icon: const Icon(Icons.close, size: 18),
                          onPressed: () =>
                              setState(() => _saveError = null),
                        ),
                      ],
                    ),
                  ),
                ),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: TextField(
                    controller: _controller,
                    maxLines: null,
                    expands: true,
                    textAlignVertical: TextAlignVertical.top,
                    style: const TextStyle(
                      fontFamily: 'monospace',
                      fontSize: 13,
                    ),
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      hintText: 'File content...',
                    ),
                    onChanged: (_) {
                      if (!_dirty) setState(() => _dirty = true);
                    },
                  ),
                ),
              ),
            ],
          );
        },
      ),
      ),  // SafeArea
    ),  // Scaffold
    );  // PopScope
  }
}
