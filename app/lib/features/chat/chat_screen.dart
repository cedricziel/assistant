import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../personas/persona_picker.dart';
import '../personas/personas_provider.dart';
import 'chat_provider.dart';
import 'conversation_list.dart';

/// Main chat screen.
///
/// - Left panel: [ConversationList] (shown as drawer on narrow screens).
/// - Right panel: message list + input row.
/// - App bar: active persona name + persona picker button.
class ChatScreen extends ConsumerStatefulWidget {
  const ChatScreen({super.key, this.conversationId});

  final String? conversationId;

  @override
  ConsumerState<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends ConsumerState<ChatScreen> {
  final _inputController = TextEditingController();
  final _scrollController = ScrollController();
  final _inputFocus = FocusNode();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadConversation();
    });
  }

  @override
  void didUpdateWidget(ChatScreen old) {
    super.didUpdateWidget(old);
    if (old.conversationId != widget.conversationId) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _loadConversation();
      });
    }
  }

  void _loadConversation() {
    final id = widget.conversationId;
    if (id != null) {
      ref.read(chatProvider.notifier).loadConversation(id);
    } else {
      ref.read(chatProvider.notifier).clearConversation();
    }
  }

  @override
  void dispose() {
    _inputController.dispose();
    _scrollController.dispose();
    _inputFocus.dispose();
    super.dispose();
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
        );
      }
    });
  }

  Future<void> _sendMessage() async {
    final text = _inputController.text.trim();
    if (text.isEmpty) return;
    _inputController.clear();
    _inputFocus.requestFocus();

    await ref.read(chatProvider.notifier).sendMessage(text);
    _scrollToBottom();
  }

  @override
  Widget build(BuildContext context) {
    final chatAsync = ref.watch(chatProvider);
    final chatState = chatAsync.value ?? const ChatState();
    final isWide = MediaQuery.of(context).size.width > 700;

    // Scroll to bottom when messages change.
    ref.listen(chatProvider, (_, next) {
      if (next.value?.messages.isNotEmpty == true) {
        _scrollToBottom();
      }
    });

    final personasAsync = ref.watch(personasProvider);
    final activePersona = personasAsync.value?.activePersona;
    final activePersonaName = activePersona?.name ?? 'Assistant';

    return Scaffold(
      appBar: AppBar(
        title: Text(activePersonaName),
        leading: isWide
            ? null
            : Builder(
                builder: (ctx) => IconButton(
                  icon: const Icon(Icons.menu),
                  onPressed: () => Scaffold.of(ctx).openDrawer(),
                ),
              ),
        actions: [
          // Persona picker button.
          IconButton(
            key: const Key('persona_picker_button'),
            icon: const Icon(Icons.switch_account_outlined),
            tooltip: 'Switch persona',
            onPressed: () => showPersonaPicker(context),
          ),
          // Navigate to traces.
          IconButton(
            icon: const Icon(Icons.timeline_outlined),
            tooltip: 'Traces',
            onPressed: () => context.go('/traces'),
          ),
          // Navigate to logs.
          IconButton(
            icon: const Icon(Icons.article_outlined),
            tooltip: 'Logs',
            onPressed: () => context.go('/logs'),
          ),
          // Navigate to skills.
          IconButton(
            icon: const Icon(Icons.extension_outlined),
            tooltip: 'Skills',
            onPressed: () => context.go('/skills'),
          ),
        ],
      ),

      // Drawer on narrow screens.
      drawer: isWide
          ? null
          : Drawer(
              child: SafeArea(
                child: ConversationList(
                  onConversationSelected: () =>
                      Navigator.of(context).pop(),
                ),
              ),
            ),

      body: GestureDetector(
        onTap: () => FocusScope.of(context).unfocus(),
        child: Row(
        children: [
          // Conversation list sidebar (wide screens only).
          if (isWide)
            SizedBox(
              width: 240,
              child: Container(
                decoration: const BoxDecoration(
                  border: Border(
                    right: BorderSide(color: Colors.black12),
                  ),
                ),
                child: const ConversationList(),
              ),
            ),

          // Chat area.
          Expanded(
            child: Column(
              children: [
                // Error banner.
                if (chatState.error != null)
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
                              chatState.error!,
                              style: TextStyle(color: Colors.red.shade700),
                            ),
                          ),
                          IconButton(
                            icon: const Icon(Icons.close, size: 18),
                            onPressed: () {
                              ref.read(chatProvider.notifier).dismissError();
                            },
                          ),
                        ],
                      ),
                    ),
                  ),

                // Loading indicator when fetching history.
                if (chatState.isLoadingHistory)
                  const LinearProgressIndicator(),

                // Message list.
                Expanded(
                  child: chatState.messages.isEmpty
                      ? const _EmptyChat()
                      : ListView.builder(
                          controller: _scrollController,
                          padding: const EdgeInsets.symmetric(
                            vertical: 16,
                            horizontal: 12,
                          ),
                          itemCount: chatState.messages.length,
                          itemBuilder: (context, index) {
                            final msg = chatState.messages[index];
                            final prevMsg = index > 0
                                ? chatState.messages[index - 1]
                                : null;
                            final isGrouped =
                                prevMsg != null &&
                                prevMsg.role == msg.role &&
                                !prevMsg.isStreaming;
                            return _MessageBubble(
                              message: msg,
                              isGrouped: isGrouped,
                            );
                          },
                        ),
                ),

                // Tool call progress indicator (shown while streaming).
                if (chatState.isSending && chatState.streamingContent.isEmpty)
                  const Padding(
                    padding: EdgeInsets.symmetric(vertical: 4),
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        SizedBox(
                          width: 14,
                          height: 14,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        ),
                        SizedBox(width: 8),
                        Text(
                          'Thinking...',
                          style: TextStyle(
                            color: Colors.black54,
                            fontSize: 13,
                          ),
                        ),
                      ],
                    ),
                  ),

                // Input row.
                _InputRow(
                  controller: _inputController,
                  focusNode: _inputFocus,
                  isSending: chatState.isSending,
                  onSend: _sendMessage,
                  onStop: () => ref.read(chatProvider.notifier).cancelStream(),
                ),
              ],
            ),
          ),
        ],
      ),
      ),  // GestureDetector
    );
  }
}

// -- Message bubble ----------------------------------------------------------

class _MessageBubble extends StatelessWidget {
  const _MessageBubble({required this.message, this.isGrouped = false});

  final ChatMessage message;
  final bool isGrouped;

  @override
  Widget build(BuildContext context) {
    final isUser = message.isUser;
    final colorScheme = Theme.of(context).colorScheme;

    return Align(
      alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        margin: EdgeInsets.only(
          top: isGrouped ? 2 : 8,
          bottom: 2,
        ),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        constraints: const BoxConstraints(maxWidth: 640),
        decoration: BoxDecoration(
          color: isUser ? colorScheme.primary : colorScheme.surfaceContainerHighest,
          borderRadius: BorderRadius.only(
            topLeft: const Radius.circular(16),
            topRight: const Radius.circular(16),
            bottomLeft: isUser
                ? const Radius.circular(16)
                : const Radius.circular(4),
            bottomRight: isUser
                ? const Radius.circular(4)
                : const Radius.circular(16),
          ),
        ),
        child: message.isStreaming && message.content.isEmpty
            ? _streamingDotsIndicator()
            : SelectableText(
                message.content,
                style: TextStyle(
                  color: isUser
                      ? colorScheme.onPrimary
                      : colorScheme.onSurface,
                ),
              ),
      ),
    );
  }

  Widget _streamingDotsIndicator() {
    return const SizedBox(
      height: 16,
      width: 40,
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceEvenly,
        children: [
          _Dot(),
          _Dot(delay: Duration(milliseconds: 150)),
          _Dot(delay: Duration(milliseconds: 300)),
        ],
      ),
    );
  }
}

class _Dot extends StatelessWidget {
  const _Dot({this.delay = Duration.zero});
  final Duration delay;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 6,
      height: 6,
      decoration: const BoxDecoration(
        color: Colors.black38,
        shape: BoxShape.circle,
      ),
    );
  }
}

// -- Empty state -------------------------------------------------------------

class _EmptyChat extends StatelessWidget {
  const _EmptyChat();

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.chat_bubble_outline, size: 64, color: Colors.black26),
          SizedBox(height: 16),
          Text(
            'Start a conversation',
            style: TextStyle(
              fontSize: 18,
              color: Colors.black45,
              fontWeight: FontWeight.w500,
            ),
          ),
          SizedBox(height: 8),
          Text(
            'Type a message below to begin.',
            style: TextStyle(color: Colors.black38),
          ),
        ],
      ),
    );
  }
}

// -- Input row ---------------------------------------------------------------

class _InputRow extends StatelessWidget {
  const _InputRow({
    required this.controller,
    required this.focusNode,
    required this.isSending,
    required this.onSend,
    required this.onStop,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool isSending;
  final VoidCallback onSend;
  final VoidCallback onStop;

  @override
  Widget build(BuildContext context) {
    final bottomInset = MediaQuery.of(context).padding.bottom;
    return Container(
      padding: EdgeInsets.fromLTRB(12, 8, 12, 12 + bottomInset),
      decoration: const BoxDecoration(
        border: Border(top: BorderSide(color: Colors.black12)),
      ),
      child: Row(
        children: [
          Expanded(
            child: TextField(
              key: const Key('message_input'),
              controller: controller,
              focusNode: focusNode,
              decoration: const InputDecoration(
                hintText: 'Type a message...',
                border: OutlineInputBorder(),
                contentPadding: EdgeInsets.symmetric(
                  horizontal: 14,
                  vertical: 10,
                ),
                isDense: true,
              ),
              minLines: 1,
              maxLines: 6,
              enabled: !isSending,
              textInputAction: TextInputAction.send,
              onSubmitted: (_) => onSend(),
            ),
          ),
          const SizedBox(width: 8),
          if (isSending)
            IconButton.filled(
              key: const Key('stop_button'),
              onPressed: onStop,
              icon: const Icon(Icons.stop_rounded),
            )
          else
            IconButton.filled(
              key: const Key('send_button'),
              onPressed: onSend,
              icon: const Icon(Icons.send),
            ),
        ],
      ),
    );
  }
}
