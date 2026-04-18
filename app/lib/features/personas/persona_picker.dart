import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:assistant_api/assistant_api.dart';

import 'personas_provider.dart';

/// Show the persona picker as a modal bottom sheet.
void showPersonaPicker(BuildContext context) {
  showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
    ),
    builder: (ctx) => const PersonaPicker(),
  );
}

/// Modal bottom sheet listing all available personas.
///
/// The active persona is marked with a checkmark.  Tapping a persona switches
/// to it via `POST /api/personas/active`.
class PersonaPicker extends ConsumerWidget {
  const PersonaPicker({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final personasAsync = ref.watch(personasProvider);

    return DraggableScrollableSheet(
      initialChildSize: 0.5,
      minChildSize: 0.3,
      maxChildSize: 0.9,
      expand: false,
      builder: (context, scrollController) {
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Handle bar
            Center(
              child: Container(
                margin: const EdgeInsets.only(top: 8, bottom: 4),
                width: 40,
                height: 4,
                decoration: BoxDecoration(
                  color: Colors.black26,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
            ),

            // Title
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 8, 20, 4),
              child: Text(
                'Choose Persona',
                style: Theme.of(context).textTheme.titleLarge,
              ),
            ),

            const Divider(),

            // Content
            Expanded(
              child: personasAsync.when(
                loading: () => const Center(child: CircularProgressIndicator()),
                error: (err, _) => Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.error_outline, color: Colors.red.shade700),
                      const SizedBox(height: 8),
                      Text(
                        'Failed to load personas',
                        style: TextStyle(color: Colors.red.shade700),
                      ),
                      TextButton(
                        onPressed: () =>
                            ref.read(personasProvider.notifier).refresh(),
                        child: const Text('Retry'),
                      ),
                    ],
                  ),
                ),
                data: (state) {
                  if (state.personas.isEmpty) {
                    return const Center(
                      child: Text(
                        'No personas available',
                        style: TextStyle(color: Colors.black45),
                      ),
                    );
                  }

                  return ListView.builder(
                    controller: scrollController,
                    itemCount: state.personas.length,
                    itemBuilder: (context, index) {
                      final persona = state.personas[index];
                      final isActive = persona.id == state.activePersona?.id;

                      return _PersonaTile(
                        persona: persona,
                        isActive: isActive,
                        isSwitching: state.isLoading,
                        onTap: () async {
                          if (!isActive) {
                            await ref
                                .read(personasProvider.notifier)
                                .switchPersona(persona.id);
                          }
                          if (context.mounted) {
                            Navigator.of(context).pop();
                          }
                        },
                      );
                    },
                  );
                },
              ),
            ),
          ],
        );
      },
    );
  }
}

class _PersonaTile extends StatelessWidget {
  const _PersonaTile({
    required this.persona,
    required this.isActive,
    required this.isSwitching,
    required this.onTap,
  });

  final PersonaSummary persona;
  final bool isActive;
  final bool isSwitching;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: CircleAvatar(
        backgroundColor: isActive
            ? Theme.of(context).colorScheme.primary
            : Colors.grey.shade200,
        child: Text(
          persona.name.isNotEmpty ? persona.name[0].toUpperCase() : '?',
          style: TextStyle(
            color: isActive ? Colors.white : Colors.black54,
            fontWeight: FontWeight.bold,
          ),
        ),
      ),
      title: Text(
        persona.name,
        style: TextStyle(
          fontWeight: isActive ? FontWeight.bold : FontWeight.normal,
        ),
      ),
      subtitle: persona.description.isNotEmpty
          ? Text(
              persona.description,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
            )
          : null,
      trailing: isActive
          ? const Icon(Icons.check_circle, color: Colors.green)
          : (isSwitching
                ? const SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : null),
      onTap: isSwitching ? null : onTap,
    );
  }
}
