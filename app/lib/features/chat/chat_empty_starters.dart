import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../personas/persona_starters_controller.dart';
import 'chat_provider.dart';

/// Empty-chat placeholder that shows the active Persona's starter prompts.
///
/// Renders the default [ThreadEmptyState] copy when the Persona has no
/// starters, or while they load or failed to load. A tap sends the prompt as
/// the first message.
class ChatEmptyStarters extends ConsumerWidget {
  const ChatEmptyStarters({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final prompts = ref.watch(activePersonaStarterPromptsProvider);
    return ThreadEmptyState(
      starters: prompts.isEmpty
          ? null
          : ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 720),
              child: StarterPrompts(
                prompts: prompts,
                onSelected: (prompt) =>
                    ref.read(chatProvider.notifier).sendMessage(prompt),
              ),
            ),
    );
  }
}
