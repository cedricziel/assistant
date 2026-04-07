import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../api/api_client.dart';
import '../connection/connection_provider.dart';
import 'personas_provider.dart';

/// Screen with a simple form for creating a new persona.
///
/// On success navigates to the persona detail screen.
class PersonaCreateScreen extends ConsumerStatefulWidget {
  const PersonaCreateScreen({super.key});

  @override
  ConsumerState<PersonaCreateScreen> createState() =>
      _PersonaCreateScreenState();
}

class _PersonaCreateScreenState extends ConsumerState<PersonaCreateScreen> {
  final _formKey = GlobalKey<FormState>();
  final _idController = TextEditingController();
  final _nameController = TextEditingController();
  final _idFocus = FocusNode();
  bool _submitting = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _idFocus.requestFocus();
    });
  }

  @override
  void dispose() {
    _idController.dispose();
    _nameController.dispose();
    _idFocus.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) return;
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return;

    setState(() {
      _submitting = true;
      _error = null;
    });

    try {
      final api = ApiClient(baseUrl: profile.baseUrl, token: profile.token);
      final personaId = _idController.text.trim();
      final personaName = _nameController.text.trim();
      await api.personas.createPersona(
        createPersonaRequest: CreatePersonaRequest((b) {
          b.id = personaId;
          b.name = personaName;
        }),
      );
      // Invalidate personas list so it reloads.
      ref.invalidate(personasProvider);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Persona "$personaName" created'),
            duration: const Duration(seconds: 2),
          ),
        );
        context.go('/contexts/${_idController.text.trim()}');
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _submitting = false;
          _error = e.toString();
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('New Persona'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/contexts'),
        ),
      ),
      body: SafeArea(
        child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Form(
          key: _formKey,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (_error != null)
                Padding(
                  padding: const EdgeInsets.only(bottom: 16),
                  child: Material(
                    color: Colors.red.shade50,
                    borderRadius: BorderRadius.circular(8),
                    child: Padding(
                      padding: const EdgeInsets.all(12),
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
                              _error!,
                              style: TextStyle(color: Colors.red.shade700),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              TextFormField(
                controller: _idController,
                focusNode: _idFocus,
                decoration: const InputDecoration(
                  labelText: 'ID',
                  hintText: 'e.g. my-persona',
                  helperText: 'Lowercase letters, numbers, hyphens, underscores.',
                  border: OutlineInputBorder(),
                ),
                validator: (v) {
                  if (v == null || v.trim().isEmpty) {
                    return 'ID is required';
                  }
                  if (!RegExp(r'^[a-z0-9_-]+$').hasMatch(v.trim())) {
                    return 'Only lowercase letters, numbers, hyphens, underscores';
                  }
                  return null;
                },
                enabled: !_submitting,
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: _nameController,
                decoration: const InputDecoration(
                  labelText: 'Name',
                  hintText: 'Display name',
                  border: OutlineInputBorder(),
                ),
                validator: (v) {
                  if (v == null || v.trim().isEmpty) {
                    return 'Name is required';
                  }
                  return null;
                },
                enabled: !_submitting,
              ),
              const SizedBox(height: 24),
              Row(
                children: [
                  Expanded(
                    child: OutlinedButton(
                      onPressed: _submitting ? null : () => context.go('/contexts'),
                      child: const Text('Cancel'),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: FilledButton(
                      onPressed: _submitting ? null : _submit,
                      child: _submitting
                          ? const SizedBox(
                              width: 18,
                              height: 18,
                              child: CircularProgressIndicator(
                                strokeWidth: 2,
                                color: Colors.white,
                              ),
                            )
                          : const Text('Create Persona'),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
        ),  // SafeArea
      ),
    );
  }
}
