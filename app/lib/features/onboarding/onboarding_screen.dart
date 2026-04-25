import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../router/app_router.dart';
import '../connection/connection_provider.dart';
import '../personas/personas_provider.dart';
import 'onboarding_provider.dart';

/// Onboarding screen shown when a user has no persona yet.
///
/// Displays available templates from the org catalog. The user picks one
/// (or creates a blank persona) and is then navigated to chat.
class OnboardingScreen extends ConsumerStatefulWidget {
  const OnboardingScreen({super.key});

  @override
  ConsumerState<OnboardingScreen> createState() => _OnboardingScreenState();
}

class _OnboardingScreenState extends ConsumerState<OnboardingScreen> {
  bool _creating = false;
  String? _error;

  Future<void> _createFromTemplate(TemplateResponse template) async {
    await _createPersona(id: template.id, name: template.name);
  }

  Future<void> _createBlank() async {
    await _createPersona(id: 'my-assistant', name: 'My Assistant');
  }

  Future<void> _createPersona({
    required String id,
    required String name,
  }) async {
    final api = ref.read(apiClientProvider);
    if (api == null) return;

    setState(() {
      _creating = true;
      _error = null;
    });

    try {
      await api.personas.createPersona(
        createPersonaRequest: CreatePersonaRequest((b) {
          b.id = id;
          b.name = name;
        }),
      );

      ref.invalidate(personasProvider);
      ref.read(onboardingStatusProvider.notifier).refresh();

      if (mounted) {
        GoRouter.of(context).go(AppRoutes.chat);
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _creating = false;
          _error = 'Failed to create persona: $e';
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final templatesAsync = ref.watch(templatesProvider);

    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 520),
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(32),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  'Welcome! Create your first persona',
                  style: Theme.of(context).textTheme.headlineMedium,
                ),
                const SizedBox(height: 8),
                Text(
                  'A persona defines how the assistant behaves. '
                  'Pick a template to get started, or create a blank one.',
                  style: Theme.of(context).textTheme.bodyMedium,
                ),
                const SizedBox(height: 24),

                if (_error != null) ...[
                  Container(
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: Theme.of(context).colorScheme.errorContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Text(
                      _error!,
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.onErrorContainer,
                      ),
                    ),
                  ),
                  const SizedBox(height: 16),
                ],

                if (_creating)
                  const Center(
                    child: Padding(
                      padding: EdgeInsets.all(32),
                      child: CircularProgressIndicator.adaptive(),
                    ),
                  )
                else
                  templatesAsync.when(
                    loading: () => const Center(
                      child: CircularProgressIndicator.adaptive(),
                    ),
                    error: (err, _) => Column(
                      children: [
                        Text('Could not load templates: $err'),
                        const SizedBox(height: 12),
                        FilledButton.tonal(
                          onPressed: () => ref.invalidate(templatesProvider),
                          child: const Text('Retry'),
                        ),
                      ],
                    ),
                    data: (templates) => Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        if (templates.isNotEmpty) ...[
                          Text(
                            'Templates',
                            style: Theme.of(context).textTheme.titleMedium,
                          ),
                          const SizedBox(height: 8),
                          for (final t in templates)
                            Card(
                              child: ListTile(
                                title: Text(t.name),
                                subtitle: t.description.isNotEmpty
                                    ? Text(
                                        t.description,
                                        maxLines: 2,
                                        overflow: TextOverflow.ellipsis,
                                      )
                                    : null,
                                trailing: const Icon(Icons.add_circle_outline),
                                onTap: () => _createFromTemplate(t),
                              ),
                            ),
                          const SizedBox(height: 16),
                          const Divider(),
                          const SizedBox(height: 16),
                        ],
                        OutlinedButton.icon(
                          onPressed: _createBlank,
                          icon: const Icon(Icons.add),
                          label: const Text('Create blank persona'),
                        ),
                      ],
                    ),
                  ),

                const SizedBox(height: 24),

                // Skip option — go straight to chat without creating.
                Center(
                  child: TextButton(
                    onPressed: _creating
                        ? null
                        : () => GoRouter.of(context).go(AppRoutes.chat),
                    child: const Text('Skip for now'),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
