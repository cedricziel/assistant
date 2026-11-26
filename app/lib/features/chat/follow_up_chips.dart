import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'chat_provider.dart';

/// Follow-up prompts offered above the composer once a turn has finished.
///
/// Renders nothing while a turn is sending or when no suggestions arrived.
/// A tap sends the prompt as the next message; the notifier clears the
/// suggestions as it queues it.
class FollowUpChips extends ConsumerWidget {
  const FollowUpChips({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final prompts = ref.watch(
      chatProvider.select((s) {
        final state = s.value;
        if (state == null || state.isSending) return const <String>[];
        return state.suggestions;
      }),
    );
    if (prompts.isEmpty) return const SizedBox.shrink();

    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 4, 16, 8),
      child: SuggestionChips(
        prompts: prompts,
        onSelected: (prompt) =>
            ref.read(chatProvider.notifier).sendMessage(prompt),
      ),
    );
  }
}
