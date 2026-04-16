import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../api/api_client.dart';
import '../../api/capabilities_provider.dart';
import '../../api/models/server_capabilities.dart';
import '../connection/connection_provider.dart';
import '../personas/persona_picker.dart';
import '../personas/personas_provider.dart';
import 'audio_player_widget.dart';
import 'chat_provider.dart';
import 'conversation_list.dart';
import 'voice_recorder_button.dart';

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

const double _kBottomThreshold = 80.0;

class _ChatScreenState extends ConsumerState<ChatScreen> {
  final _inputController = TextEditingController();
  final _scrollController = ScrollController();
  final _inputFocus = FocusNode();
  bool _atBottom = true;

  @override
  void initState() {
    super.initState();
    _inputFocus.addListener(_onInputFocusChange);
    _scrollController.addListener(_onScroll);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadConversation();
    });
  }

  /// Scrolls to the bottom when the input gains focus, so the latest messages
  /// remain visible above the keyboard on mobile (especially iOS Safari).
  /// Only scrolls if the list is already near the bottom — if the user has
  /// scrolled up to read history we leave their position alone.
  void _onInputFocusChange() {
    if (!_inputFocus.hasFocus) return;
    // Delay past the iOS keyboard animation (~300 ms) before measuring/scrolling.
    Future.delayed(const Duration(milliseconds: 350), () {
      if (!mounted || !_scrollController.hasClients) return;
      final pos = _scrollController.position;
      if (pos.maxScrollExtent - pos.pixels < 200) {
        _scrollController.animateTo(
          pos.maxScrollExtent,
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
        );
      }
    });
  }

  void _onScroll() {
    if (!_scrollController.hasClients) return;
    final pos = _scrollController.position;
    final nearBottom = pos.pixels >= pos.maxScrollExtent - _kBottomThreshold;
    if (_atBottom != nearBottom) setState(() => _atBottom = nearBottom);
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
    _scrollToBottom();
  }

  /// Retry loading the conversation once the API client becomes available.
  ///
  /// On a hard reload / deep link, [_loadConversation] fires from [initState]
  /// before the active context has finished loading, so [loadConversation]
  /// finds no API client and returns immediately.  This listener fires when
  /// [apiClientProvider] transitions null → non-null (initial context load) or
  /// switches to a different client (context switch) and retriggers the load
  /// if the conversation hasn't been set on the chat state yet.
  void _onApiClientAvailable(ApiClient? prev, ApiClient? next) {
    if (next == null || identical(prev, next)) return;
    final id = widget.conversationId;
    if (id == null) return;
    final chatState = ref.read(chatProvider).value;
    if (prev != null || chatState?.conversationId != id) {
      _loadConversation();
    }
  }

  @override
  void dispose() {
    _inputFocus.removeListener(_onInputFocusChange);
    _scrollController.removeListener(_onScroll);
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

  Future<void> _sendVoiceMessage(Uint8List bytes, String mimeType) async {
    await ref.read(chatProvider.notifier).sendVoiceMessage(bytes, mimeType);
    _scrollToBottom();
  }

  @override
  Widget build(BuildContext context) {
    // Retry loading the conversation once the API client becomes available.
    // Handles deep-link / hard-reload races where the active context loads
    // after the first _loadConversation() call.
    ref.listen<ApiClient?>(apiClientProvider, _onApiClientAvailable);

    final chatAsync = ref.watch(chatProvider);
    final chatState = chatAsync.value ?? const ChatState();
    final isWide = MediaQuery.of(context).size.width > 700;

    // Scroll to bottom when messages change, but only if already at bottom.
    ref.listen(chatProvider, (_, next) {
      if (_atBottom && next.value?.messages.isNotEmpty == true) {
        _scrollToBottom();
      }
    });

    final personasAsync = ref.watch(personasProvider);
    final activePersona = personasAsync.value?.activePersona;
    final activePersonaName = activePersona?.name ?? 'Assistant';

    // Resolve capabilities only when an API client is available (avoids
    // triggering async work in tests that have no active profile).
    final hasClient = ref.watch(apiClientProvider) != null;
    final capabilities = hasClient
        ? (ref.watch(capabilitiesProvider).value ?? ServerCapabilities.disabled)
        : ServerCapabilities.disabled;

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
                  onConversationSelected: () => Navigator.of(context).pop(),
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
                    border: Border(right: BorderSide(color: Colors.black12)),
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
                    child: Stack(
                      children: [
                        chatState.messages.isEmpty
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
                                    capabilities: capabilities,
                                    onRetry: msg.status == MessageStatus.failed
                                        ? () => ref
                                              .read(chatProvider.notifier)
                                              .retryMessage(msg)
                                        : null,
                                    fetchMessageAudio: () {
                                      final api = ref.read(apiClientProvider);
                                      // Prefer pre-synthesized audio from the
                                      // audio_ready SSE event; fall back to
                                      // on-demand synthesis via the message ID.
                                      final audioId = msg.audioId;
                                      if (audioId != null) {
                                        return api?.fetchAudio(audioId) ??
                                            Future.value(null);
                                      }
                                      return api?.fetchMessageAudio(msg.id) ??
                                          Future.value(null);
                                    },
                                  );
                                },
                              ),
                        if (!_atBottom)
                          Positioned(
                            bottom: 8,
                            right: 8,
                            child: FloatingActionButton.small(
                              key: const Key('scroll_to_bottom_button'),
                              onPressed: _scrollToBottom,
                              tooltip: 'Scroll to bottom',
                              child: const Icon(Icons.keyboard_arrow_down),
                            ),
                          ),
                      ],
                    ),
                  ),

                  // Progress indicator (shown while streaming).
                  if (chatState.isSending && chatState.streamingContent.isEmpty)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 4),
                      child: Row(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          const SizedBox(
                            width: 14,
                            height: 14,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          ),
                          const SizedBox(width: 8),
                          Text(
                            chatState.statusMessage ?? 'Thinking...',
                            style: const TextStyle(
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
                    pendingQueueCount: chatState.pendingQueue.length,
                    capabilities: capabilities,
                    onSend: _sendMessage,
                    onStop: () =>
                        ref.read(chatProvider.notifier).cancelStream(),
                    onVoiceRecorded: _sendVoiceMessage,
                  ),
                ],
              ),
            ),
          ],
        ),
      ), // GestureDetector
    );
  }
}

// -- Message bubble ----------------------------------------------------------

class _MessageBubble extends StatelessWidget {
  const _MessageBubble({
    required this.message,
    required this.capabilities,
    required this.fetchMessageAudio,
    this.isGrouped = false,
    this.onRetry,
  });

  final ChatMessage message;
  final bool isGrouped;
  final VoidCallback? onRetry;
  final ServerCapabilities capabilities;
  final Future<Uint8List?> Function() fetchMessageAudio;

  @override
  Widget build(BuildContext context) {
    final isUser = message.isUser;
    final colorScheme = Theme.of(context).colorScheme;
    final isFailed = message.status == MessageStatus.failed;

    return Align(
      alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
      child: Column(
        crossAxisAlignment: isUser
            ? CrossAxisAlignment.end
            : CrossAxisAlignment.start,
        children: [
          Container(
            margin: EdgeInsets.only(top: isGrouped ? 2 : 8, bottom: 2),
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
            constraints: const BoxConstraints(maxWidth: 640),
            decoration: BoxDecoration(
              color: isUser
                  ? colorScheme.primary
                  : colorScheme.surfaceContainerHighest,
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
              border: isFailed
                  ? Border.all(color: Colors.red.shade400, width: 1.5)
                  : null,
            ),
            child: message.isStreaming && message.content.isEmpty
                ? _streamingDotsIndicator()
                : isUser
                ? Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (isFailed)
                        Padding(
                          padding: const EdgeInsets.only(right: 6),
                          child: Icon(
                            Icons.error_outline,
                            size: 16,
                            color: Colors.red.shade300,
                          ),
                        ),
                      Flexible(
                        child: SelectableText(
                          message.content,
                          style: TextStyle(color: colorScheme.onPrimary),
                        ),
                      ),
                    ],
                  )
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      MarkdownBody(
                        data: message.content,
                        styleSheet:
                            MarkdownStyleSheet.fromTheme(
                              Theme.of(context),
                            ).copyWith(
                              p: TextStyle(color: colorScheme.onSurface),
                              code: TextStyle(
                                color: colorScheme.onSurface,
                                backgroundColor:
                                    colorScheme.surfaceContainerLowest,
                              ),
                            ),
                        selectable: true,
                      ),
                      // Play button for assistant messages. Shows whenever
                      // voice is enabled — fetches on-demand if no audioId.
                      if (capabilities.voiceReceive && !message.isStreaming)
                        Padding(
                          padding: const EdgeInsets.only(top: 6),
                          child: AudioPlayerWidget(
                            fetchAudio: fetchMessageAudio,
                          ),
                        ),
                    ],
                  ),
          ),
          // Retry button shown below the bubble for failed user messages.
          if (isFailed && onRetry != null)
            Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: TextButton.icon(
                key: const Key('retry_button'),
                onPressed: onRetry,
                icon: const Icon(Icons.refresh, size: 14),
                label: const Text('Retry'),
                style: TextButton.styleFrom(
                  foregroundColor: Colors.red.shade700,
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 2,
                  ),
                  minimumSize: Size.zero,
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  textStyle: const TextStyle(fontSize: 12),
                ),
              ),
            ),
        ],
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
    required this.pendingQueueCount,
    required this.capabilities,
    required this.onSend,
    required this.onStop,
    required this.onVoiceRecorded,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool isSending;
  final int pendingQueueCount;
  final ServerCapabilities capabilities;
  final VoidCallback onSend;
  final VoidCallback onStop;
  final void Function(Uint8List bytes, String mimeType) onVoiceRecorded;

  @override
  Widget build(BuildContext context) {
    final bottomInset = MediaQuery.of(context).padding.bottom;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Queue depth badge — visible when messages are waiting.
        if (pendingQueueCount > 0)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
            color: Colors.amber.shade50,
            child: Row(
              children: [
                Icon(
                  Icons.hourglass_top_rounded,
                  size: 14,
                  color: Colors.amber.shade800,
                ),
                const SizedBox(width: 6),
                Text(
                  '$pendingQueueCount message${pendingQueueCount == 1 ? '' : 's'} queued',
                  key: const Key('queue_depth_badge'),
                  style: TextStyle(fontSize: 12, color: Colors.amber.shade800),
                ),
              ],
            ),
          ),
        Container(
          padding: EdgeInsets.fromLTRB(12, 8, 12, 12 + bottomInset),
          decoration: const BoxDecoration(
            border: Border(top: BorderSide(color: Colors.black12)),
          ),
          child: Row(
            children: [
              // Voice recorder button — shown when server supports voice send.
              if (capabilities.voiceSend && !isSending)
                VoiceRecorderButton(
                  onRecordingComplete: onVoiceRecorded,
                  onError: (err) {
                    ScaffoldMessenger.of(
                      context,
                    ).showSnackBar(SnackBar(content: Text(err)));
                  },
                ),
              if (capabilities.voiceSend && !isSending)
                const SizedBox(width: 4),
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
        ),
      ],
    );
  }
}
