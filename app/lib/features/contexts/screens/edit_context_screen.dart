import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/context_model.dart';
import '../providers/context_providers.dart';

/// Screen for editing an existing [AssistantContext].
///
/// Pre-fills all fields from [existingContext] and saves on confirm.
class EditContextScreen extends ConsumerStatefulWidget {
  const EditContextScreen({super.key, required this.existingContext});

  final AssistantContext existingContext;

  @override
  ConsumerState<EditContextScreen> createState() => _EditContextScreenState();
}

class _EditContextScreenState extends ConsumerState<EditContextScreen> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _nameCtrl;
  late final TextEditingController _urlCtrl;
  late final TextEditingController _tokenCtrl;
  bool _tokenVisible = false;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _nameCtrl = TextEditingController(text: widget.existingContext.name);
    _urlCtrl = TextEditingController(text: widget.existingContext.serverUrl);
    _tokenCtrl = TextEditingController(
      text: widget.existingContext.authToken ?? '',
    );
  }

  @override
  void dispose() {
    _nameCtrl.dispose();
    _urlCtrl.dispose();
    _tokenCtrl.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;
    setState(() => _saving = true);
    try {
      final token = _tokenCtrl.text.trim();
      final updated = widget.existingContext.copyWith(
        name: _nameCtrl.text.trim(),
        serverUrl: _urlCtrl.text.trim().replaceAll(RegExp(r'/$'), ''),
        authToken: token.isEmpty ? null : token,
        clearAuthToken: token.isEmpty,
      );
      await ref.read(contextsProvider.notifier).saveContext(updated);

      // If the edited context is active, refresh it.
      final active = ref.read(activeContextProvider).value;
      if (active?.id == updated.id) {
        await ref.read(activeContextProvider.notifier).activate(updated);
      }

      if (mounted) Navigator.of(context).pop();
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  String? _validateName(String? value) {
    if (value == null || value.trim().isEmpty) return 'Name is required';
    final existing = ref.read(contextsProvider).value ?? [];
    final duplicate = existing.any(
      (c) =>
          c.id != widget.existingContext.id &&
          c.name.toLowerCase() == value.trim().toLowerCase(),
    );
    if (duplicate) return 'A context with this name already exists';
    return null;
  }

  String? _validateUrl(String? value) {
    if (value == null || value.trim().isEmpty) return 'URL is required';
    final uri = Uri.tryParse(value.trim());
    if (uri == null || !uri.hasScheme || !uri.hasAuthority) {
      return 'Enter a valid URL (e.g. http://localhost:8080)';
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Edit Context'),
        actions: [
          TextButton(
            onPressed: _saving ? null : _save,
            child: const Text('Save'),
          ),
        ],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: Form(
          key: _formKey,
          child: Column(
            children: [
              TextFormField(
                controller: _nameCtrl,
                decoration: const InputDecoration(labelText: 'Name'),
                validator: _validateName,
                textInputAction: TextInputAction.next,
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: _urlCtrl,
                decoration: const InputDecoration(
                  labelText: 'URL',
                  hintText: 'https://assistant.example.com',
                ),
                validator: _validateUrl,
                keyboardType: TextInputType.url,
                textInputAction: TextInputAction.next,
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: _tokenCtrl,
                decoration: InputDecoration(
                  labelText: 'Token (optional)',
                  suffixIcon: IconButton(
                    icon: Icon(
                      _tokenVisible ? Icons.visibility_off : Icons.visibility,
                    ),
                    onPressed: () =>
                        setState(() => _tokenVisible = !_tokenVisible),
                  ),
                ),
                obscureText: !_tokenVisible,
                textInputAction: TextInputAction.done,
                onFieldSubmitted: (_) => _save(),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
