import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import 'persona_starters_controller.dart';

/// "Starter prompts" section of the Persona detail screen. Lists the prompts
/// and offers the editor only when the caller may edit them.
class PersonaStartersSection extends ConsumerWidget {
  const PersonaStartersSection({super.key, required this.personaId});

  final String personaId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final view = ref.watch(personaStartersProvider(personaId)).value;
    if (view == null) return const SizedBox.shrink();

    return Card(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          ListTile(
            title: const Text('Starter prompts'),
            subtitle: Text('${view.prompts.length} of $kMaxStarterPrompts'),
            trailing: view.canEdit
                ? TextButton(
                    key: const Key('starters_edit'),
                    onPressed: () =>
                        context.go('/personas/$personaId/starters'),
                    child: const Text('Edit'),
                  )
                : null,
          ),
          if (view.prompts.isEmpty)
            const Padding(
              padding: EdgeInsets.fromLTRB(16, 0, 16, 16),
              child: Text('No starter prompts'),
            )
          else
            for (final prompt in view.prompts)
              ListTile(dense: true, title: Text(prompt)),
        ],
      ),
    );
  }
}
