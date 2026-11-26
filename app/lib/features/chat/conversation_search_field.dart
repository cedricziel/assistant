import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../shared/platform/adaptive_text_field.dart';
import 'conversation_search_provider.dart';

/// The sidebar's conversation search box.
///
/// Writes what the user types to [conversationSearchQueryProvider], which is
/// the single source of truth: clearing the query from elsewhere (for example
/// after a result is opened) empties the field too. A clear button appears
/// while there is text, and Esc clears it.
class ConversationSearchField extends ConsumerStatefulWidget {
  const ConversationSearchField({super.key});

  @override
  ConsumerState<ConversationSearchField> createState() =>
      _ConversationSearchFieldState();
}

class _ConversationSearchFieldState
    extends ConsumerState<ConversationSearchField> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _clear() => ref.read(conversationSearchQueryProvider.notifier).clear();

  @override
  Widget build(BuildContext context) {
    ref.listen(conversationSearchQueryProvider, (_, query) {
      if (_controller.text != query) _controller.text = query;
    });
    final hasText = ref.watch(conversationSearchQueryProvider).isNotEmpty;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: CallbackShortcuts(
        bindings: {const SingleActivator(LogicalKeyboardKey.escape): _clear},
        child: AdaptiveTextField(
          key: const Key('conversation_search_field'),
          controller: _controller,
          placeholder: 'Search conversations',
          textInputAction: TextInputAction.search,
          prefix: const Icon(Icons.search, size: 18),
          suffix: hasText
              ? IconButton(
                  key: const Key('conversation_search_clear'),
                  tooltip: 'Clear search',
                  icon: const Icon(Icons.close, size: 18),
                  onPressed: _clear,
                )
              : null,
          onChanged: (value) =>
              ref.read(conversationSearchQueryProvider.notifier).set(value),
        ),
      ),
    );
  }
}
