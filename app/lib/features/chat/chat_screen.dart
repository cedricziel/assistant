import 'dart:async';

import 'package:audioplayers/audioplayers.dart';
import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:cached_network_image/cached_network_image.dart';
import 'package:desktop_drop/desktop_drop.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../api/api_client.dart';
import '../../api/attachment_service.dart';
import '../../api/capabilities_provider.dart';
import '../../api/models/server_capabilities.dart';
import '../connection/connection_provider.dart';
import '../personas/persona_picker.dart';
import '../personas/personas_provider.dart';
import 'attachment_provider.dart';
import 'audio_player_widget.dart';
import 'chat_provider.dart';
import 'conversation_list.dart';
import 'tool_call_chip.dart';
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
    final pending = ref.read(pendingAttachmentsProvider);
    if (text.isEmpty && pending.isEmpty) return;
    _inputController.clear();
    _inputFocus.requestFocus();

    // Upload pending attachments first, then send with IDs.
    final attachmentIds = <String>[];
    if (pending.isNotEmpty) {
      ref.read(pendingAttachmentsProvider.notifier).clear();
      final api = ref.read(apiClientProvider);
      if (api != null) {
        final chatState = ref.read(chatProvider).value ?? const ChatState();
        var conversationId = chatState.conversationId;
        // Create conversation if needed (mirrors ChatNotifier.sendMessage).
        if (conversationId == null) {
          final response = await api.conversations.createConversation(
            createConversationRequest: CreateConversationRequest((b) => b),
          );
          final conv = response.data!;
          conversationId = conv.id;
          ref.read(conversationListProvider.notifier).prependConversation(conv);
          ref.read(chatProvider.notifier).setConversationId(conversationId);
        }
        final service = AttachmentService(api.attachments);
        for (final p in pending) {
          try {
            final meta = await service.upload(
              conversationId: conversationId,
              bytes: p.bytes,
              filename: p.filename,
              mimeType: p.mimeType,
            );
            attachmentIds.add(meta.id);
          } catch (e) {
            // Skip failed uploads — don't block sending the message.
          }
        }
      }
    }

    final msg = text.isNotEmpty ? text : '[Image attached]';
    await ref
        .read(chatProvider.notifier)
        .sendMessage(msg, attachmentIds: attachmentIds);
    _scrollToBottom();
  }

  Future<void> _sendVoiceMessage(Uint8List bytes, String mimeType) async {
    await ref.read(chatProvider.notifier).sendVoiceMessage(bytes, mimeType);
    _scrollToBottom();
  }

  Future<void> _pickImages() async {
    final result = await FilePicker.pickFiles(
      type: FileType.image,
      allowMultiple: true,
      withData: true,
    );
    if (result == null) return;
    final notifier = ref.read(pendingAttachmentsProvider.notifier);
    for (final file in result.files) {
      if (file.bytes != null) {
        final mime = _mimeFromExtension(file.extension);
        notifier.add(
          PendingAttachment(
            bytes: file.bytes!,
            filename: file.name,
            mimeType: mime,
          ),
        );
      }
    }
  }

  static String _mimeFromExtension(String? ext) {
    return switch (ext?.toLowerCase()) {
      'png' => 'image/png',
      'jpg' || 'jpeg' => 'image/jpeg',
      'gif' => 'image/gif',
      'webp' => 'image/webp',
      _ => 'application/octet-stream',
    };
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

            // Chat area with drop target for image attachments.
            Expanded(
              child: DropTarget(
                onDragDone: (details) {
                  final notifier = ref.read(
                    pendingAttachmentsProvider.notifier,
                  );
                  for (final file in details.files) {
                    file.readAsBytes().then((bytes) {
                      final ext = file.name.split('.').last;
                      final mime = _mimeFromExtension(ext);
                      if (mime.startsWith('image/')) {
                        notifier.add(
                          PendingAttachment(
                            bytes: bytes,
                            filename: file.name,
                            mimeType: mime,
                          ),
                        );
                      }
                    });
                  }
                },
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
                                  ref
                                      .read(chatProvider.notifier)
                                      .dismissError();
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
                                      onRetry:
                                          msg.status == MessageStatus.failed
                                          ? () => ref
                                                .read(chatProvider.notifier)
                                                .retryMessage(msg)
                                          : null,
                                      onStop: () => ref
                                          .read(chatProvider.notifier)
                                          .cancelStream(),
                                      fetchMessageAudio: () {
                                        final api = ref.read(apiClientProvider);
                                        final audioId = msg.audioId;
                                        if (audioId != null) {
                                          return api?.fetchAudio(audioId) ??
                                              Future.value(null);
                                        }
                                        // On-demand TTS synthesis.
                                        return api?.fetchMessageAudio(msg.id) ??
                                            Future.value(null);
                                      },
                                      imageBaseUrl: ref
                                          .read(activeProfileProvider)
                                          ?.baseUrl,
                                      imageAuthToken: ref
                                          .read(activeProfileProvider)
                                          ?.token,
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

                    // Input row.
                    _InputRow(
                      controller: _inputController,
                      focusNode: _inputFocus,
                      isSending: chatState.isSending,
                      pendingQueueCount: chatState.pendingQueue.length,
                      pendingAttachments: ref.watch(pendingAttachmentsProvider),
                      capabilities: capabilities,
                      onSend: _sendMessage,
                      onStop: () =>
                          ref.read(chatProvider.notifier).cancelStream(),
                      onVoiceRecorded: _sendVoiceMessage,
                      onPickImage: _pickImages,
                      onRemoveAttachment: (i) => ref
                          .read(pendingAttachmentsProvider.notifier)
                          .removeAt(i),
                      onPasteImage: (bytes) {
                        ref
                            .read(pendingAttachmentsProvider.notifier)
                            .add(
                              PendingAttachment(
                                bytes: bytes,
                                filename: 'pasted_image.png',
                                mimeType: 'image/png',
                              ),
                            );
                      },
                    ),
                  ],
                ),
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
    this.onStop,
    this.imageBaseUrl,
    this.imageAuthToken,
  });

  final ChatMessage message;
  final bool isGrouped;
  final VoidCallback? onRetry;
  final VoidCallback? onStop;
  final ServerCapabilities capabilities;
  final Future<Uint8List?> Function() fetchMessageAudio;
  final String? imageBaseUrl;
  final String? imageAuthToken;

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
                ? Column(
                    crossAxisAlignment: CrossAxisAlignment.end,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (message.attachments.isNotEmpty)
                        _attachmentThumbnails(context),
                      Row(
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
                      ),
                    ],
                  )
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      // Tool call chips — one per invocation, in order.
                      if (message.toolCalls.isNotEmpty)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 4),
                          child: Wrap(
                            spacing: 4,
                            runSpacing: 4,
                            children: message.toolCalls
                                .map((tc) => ToolCallChip(record: tc))
                                .toList(),
                          ),
                        ),
                      // Divider between chips and reply text.
                      if (message.toolCalls.isNotEmpty &&
                          message.content.isNotEmpty)
                        const Padding(
                          padding: EdgeInsets.only(bottom: 6),
                          child: Divider(height: 1, thickness: 0.5),
                        ),
                      if (message.isStreaming && message.tokenStream != null)
                        StreamMarkdown(
                          stream: message.tokenStream!,
                          styleSheet:
                              MarkdownStyleSheet.fromTheme(
                                Theme.of(context),
                              ).copyWith(
                                paragraphStyle: TextStyle(
                                  color: colorScheme.onSurface,
                                ),
                                inlineCodeStyle: TextStyle(
                                  color: colorScheme.onSurface,
                                  backgroundColor:
                                      colorScheme.surfaceContainerLowest,
                                ),
                              ),
                          useEnhancedComponents: true,
                          plugins: ParserPluginRegistry()
                            ..registerBlock(MermaidPlugin()),
                          builderRegistry: BuilderRegistry()
                            ..register('mermaid', const MermaidBuilder()),
                        )
                      else
                        SmoothMarkdown(
                          data: message.content,
                          styleSheet:
                              MarkdownStyleSheet.fromTheme(
                                Theme.of(context),
                              ).copyWith(
                                paragraphStyle: TextStyle(
                                  color: colorScheme.onSurface,
                                ),
                                inlineCodeStyle: TextStyle(
                                  color: colorScheme.onSurface,
                                  backgroundColor:
                                      colorScheme.surfaceContainerLowest,
                                ),
                              ),
                          selectable: true,
                          useEnhancedComponents: true,
                          plugins: ParserPluginRegistry()
                            ..registerBlock(MermaidPlugin()),
                          builderRegistry: BuilderRegistry()
                            ..register('mermaid', const MermaidBuilder()),
                        ),
                      // Attachment thumbnails (assistant-produced images).
                      if (message.attachments.isNotEmpty)
                        _attachmentThumbnails(context),
                      // Inline audio player for messages with real audio
                      // (agent intentionally produced voice via AudioReadyEvent).
                      if (message.audioId != null && !message.isStreaming)
                        Padding(
                          padding: const EdgeInsets.only(top: 6),
                          child: AudioPlayerWidget(
                            fetchAudio: fetchMessageAudio,
                          ),
                        ),
                    ],
                  ),
          ),
          // Meta action row below the bubble.
          _MetaActionRow(
            message: message,
            capabilities: capabilities,
            fetchMessageAudio: fetchMessageAudio,
            onRetry: onRetry,
            onStop: onStop,
          ),
        ],
      ),
    );
  }

  Widget _attachmentThumbnails(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isUser = message.isUser;
    return Padding(
      padding: EdgeInsets.only(bottom: isUser ? 6 : 0, top: isUser ? 0 : 6),
      child: Wrap(
        spacing: 6,
        runSpacing: 6,
        children: message.attachments.map((att) {
          final thumbUrl = imageBaseUrl != null
              ? '$imageBaseUrl${att.url}?w=300'
              : '${att.url}?w=300';
          return GestureDetector(
            onTap: () => _showFullImage(context, att),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(8),
              child: CachedNetworkImage(
                imageUrl: thumbUrl,
                width: 150,
                height: 150,
                fit: BoxFit.cover,
                httpHeaders: imageAuthToken != null
                    ? {'Authorization': 'Bearer $imageAuthToken'}
                    : const {},
                placeholder: (_, _) => Container(
                  width: 150,
                  height: 150,
                  color: colorScheme.surfaceContainerLowest,
                  child: const Center(
                    child: SizedBox(
                      width: 24,
                      height: 24,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  ),
                ),
                errorWidget: (_, _, _) => Container(
                  width: 150,
                  height: 150,
                  color: colorScheme.surfaceContainerLowest,
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(Icons.broken_image, color: colorScheme.outline),
                      const SizedBox(height: 4),
                      Text(
                        att.filename,
                        style: TextStyle(
                          fontSize: 10,
                          color: colorScheme.outline,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
              ),
            ),
          );
        }).toList(),
      ),
    );
  }

  void _showFullImage(BuildContext context, ChatAttachment attachment) {
    final fullUrl = imageBaseUrl != null
        ? '$imageBaseUrl${attachment.url}?w=1920'
        : '${attachment.url}?w=1920';
    showDialog(
      context: context,
      builder: (ctx) => Dialog(
        backgroundColor: Colors.transparent,
        insetPadding: const EdgeInsets.all(16),
        child: Stack(
          alignment: Alignment.topRight,
          children: [
            InteractiveViewer(
              child: CachedNetworkImage(
                imageUrl: fullUrl,
                fit: BoxFit.contain,
                httpHeaders: imageAuthToken != null
                    ? {'Authorization': 'Bearer $imageAuthToken'}
                    : const {},
                placeholder: (_, _) =>
                    const Center(child: CircularProgressIndicator()),
                errorWidget: (_, _, _) => const Center(
                  child: Icon(
                    Icons.broken_image,
                    size: 64,
                    color: Colors.white70,
                  ),
                ),
              ),
            ),
            IconButton(
              icon: const Icon(Icons.close, color: Colors.white),
              onPressed: () => Navigator.of(ctx).pop(),
            ),
          ],
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

// -- Meta action row ---------------------------------------------------------

/// Contextual action row rendered below each message bubble.
///
/// Shows actions like Copy, Read aloud, Retry, and Stop depending on
/// the message type and state.
class _MetaActionRow extends StatelessWidget {
  const _MetaActionRow({
    required this.message,
    required this.capabilities,
    required this.fetchMessageAudio,
    this.onRetry,
    this.onStop,
  });

  final ChatMessage message;
  final ServerCapabilities capabilities;
  final Future<Uint8List?> Function() fetchMessageAudio;
  final VoidCallback? onRetry;
  final VoidCallback? onStop;

  @override
  Widget build(BuildContext context) {
    final isUser = message.isUser;
    final isFailed = message.status == MessageStatus.failed;
    final colorScheme = Theme.of(context).colorScheme;
    final mutedColor = colorScheme.onSurface.withValues(alpha: 0.6);

    if (message.isStreaming) {
      if (onStop == null) return const SizedBox.shrink();
      return Padding(
        padding: const EdgeInsets.only(top: 4),
        child: _MetaActionButton(
          key: const Key('stop_action'),
          icon: Icons.stop_rounded,
          label: 'Stop',
          color: mutedColor,
          onTap: onStop!,
        ),
      );
    }

    final actions = <Widget>[];

    // Read aloud: assistant messages without real audio, when TTS is available.
    if (!isUser &&
        capabilities.voiceReceive &&
        message.audioId == null &&
        message.content.isNotEmpty) {
      actions.add(
        _ReadAloudAction(
          key: const Key('read_aloud_action'),
          fetchAudio: fetchMessageAudio,
          color: mutedColor,
        ),
      );
    }

    // Copy: all messages with content.
    if (message.content.isNotEmpty) {
      actions.add(
        _MetaActionButton(
          key: const Key('copy_action'),
          icon: Icons.copy_outlined,
          label: 'Copy',
          color: mutedColor,
          onTap: () {
            Clipboard.setData(ClipboardData(text: message.content));
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(
                content: Text('Copied to clipboard'),
                duration: Duration(seconds: 1),
              ),
            );
          },
        ),
      );
    }

    // Retry: failed messages only.
    if (isFailed && onRetry != null) {
      actions.add(
        _MetaActionButton(
          key: const Key('retry_button'),
          icon: Icons.refresh,
          label: 'Retry',
          color: Colors.red.shade700,
          onTap: onRetry!,
        ),
      );
    }

    if (actions.isEmpty) return const SizedBox.shrink();

    return Padding(
      padding: const EdgeInsets.only(top: 4),
      child: Row(
        mainAxisAlignment: isUser
            ? MainAxisAlignment.end
            : MainAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          for (int i = 0; i < actions.length; i++) ...[
            if (i > 0) const SizedBox(width: 16),
            actions[i],
          ],
        ],
      ),
    );
  }
}

/// A small icon + label button used in the meta action row.
class _MetaActionButton extends StatelessWidget {
  const _MetaActionButton({
    super.key,
    required this.icon,
    required this.label,
    required this.onTap,
    required this.color,
  });

  final IconData icon;
  final String label;
  final VoidCallback onTap;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(4),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 14, color: color),
            const SizedBox(width: 4),
            Text(label, style: TextStyle(fontSize: 12, color: color)),
          ],
        ),
      ),
    );
  }
}

// -- Read aloud action -------------------------------------------------------

/// State for the read-aloud TTS action.
enum _ReadAloudState { idle, loading, playing, error }

/// Stateful meta action that fetches on-demand TTS and plays it back.
class _ReadAloudAction extends StatefulWidget {
  const _ReadAloudAction({
    super.key,
    required this.fetchAudio,
    required this.color,
  });

  final Future<Uint8List?> Function() fetchAudio;
  final Color color;

  @override
  State<_ReadAloudAction> createState() => _ReadAloudActionState();
}

class _ReadAloudActionState extends State<_ReadAloudAction> {
  final _player = AudioPlayer();
  _ReadAloudState _state = _ReadAloudState.idle;
  Uint8List? _cachedBytes;
  Timer? _errorTimer;

  @override
  void initState() {
    super.initState();
    _player.onPlayerComplete.listen((_) {
      if (mounted) setState(() => _state = _ReadAloudState.idle);
    });
  }

  @override
  void dispose() {
    _errorTimer?.cancel();
    _player.dispose();
    super.dispose();
  }

  Future<void> _toggle() async {
    switch (_state) {
      case _ReadAloudState.playing:
        await _player.stop();
        setState(() => _state = _ReadAloudState.idle);
      case _ReadAloudState.idle:
      case _ReadAloudState.error:
        setState(() => _state = _ReadAloudState.loading);
        try {
          _cachedBytes ??= await widget.fetchAudio();
          final bytes = _cachedBytes;
          if (bytes == null || !mounted) {
            _showError();
            return;
          }
          await _player.play(BytesSource(bytes));
          if (mounted) setState(() => _state = _ReadAloudState.playing);
        } catch (_) {
          _showError();
        }
      case _ReadAloudState.loading:
        break; // ignore taps while loading
    }
  }

  void _showError() {
    if (!mounted) return;
    setState(() => _state = _ReadAloudState.error);
    _errorTimer?.cancel();
    _errorTimer = Timer(const Duration(seconds: 4), () {
      if (mounted) setState(() => _state = _ReadAloudState.idle);
    });
  }

  @override
  Widget build(BuildContext context) {
    final errorColor = Theme.of(context).colorScheme.error;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        switch (_state) {
          _ReadAloudState.idle => _MetaActionButton(
            icon: Icons.volume_up_outlined,
            label: 'Read aloud',
            color: widget.color,
            onTap: _toggle,
          ),
          _ReadAloudState.loading => Padding(
            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                SizedBox(
                  width: 14,
                  height: 14,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: widget.color,
                  ),
                ),
                const SizedBox(width: 4),
                Text(
                  'Loading\u2026',
                  style: TextStyle(fontSize: 12, color: widget.color),
                ),
              ],
            ),
          ),
          _ReadAloudState.playing => _MetaActionButton(
            icon: Icons.stop_rounded,
            label: 'Stop reading',
            color: widget.color,
            onTap: _toggle,
          ),
          _ReadAloudState.error => _MetaActionButton(
            icon: Icons.volume_up_outlined,
            label: 'Read aloud',
            color: widget.color,
            onTap: _toggle,
          ),
        },
        if (_state == _ReadAloudState.error)
          Padding(
            padding: const EdgeInsets.only(top: 2, left: 4),
            child: Text(
              'Could not generate audio',
              style: TextStyle(fontSize: 11, color: errorColor),
            ),
          ),
      ],
    );
  }
}

class _Dot extends StatefulWidget {
  const _Dot({this.delay = Duration.zero});
  final Duration delay;

  @override
  State<_Dot> createState() => _DotState();
}

class _DotState extends State<_Dot> with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _opacity;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      duration: const Duration(milliseconds: 1200),
      vsync: this,
    );

    // Stagger offset as a fraction of total duration.
    final delay = widget.delay.inMilliseconds / 1200;
    final end = (delay + 0.5).clamp(0.0, 1.0);

    _opacity =
        TweenSequence<double>([
          TweenSequenceItem(
            tween: Tween(
              begin: 0.2,
              end: 1.0,
            ).chain(CurveTween(curve: Curves.easeInOut)),
            weight: 50,
          ),
          TweenSequenceItem(
            tween: Tween(
              begin: 1.0,
              end: 0.2,
            ).chain(CurveTween(curve: Curves.easeInOut)),
            weight: 50,
          ),
        ]).animate(
          CurvedAnimation(parent: _controller, curve: Interval(delay, end)),
        );

    _controller.repeat();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final color = Theme.of(context).colorScheme.onSurfaceVariant;

    if (MediaQuery.of(context).disableAnimations) {
      return Container(
        width: 6,
        height: 6,
        decoration: BoxDecoration(color: color, shape: BoxShape.circle),
      );
    }

    return AnimatedBuilder(
      animation: _opacity,
      builder: (context, child) =>
          Opacity(opacity: _opacity.value, child: child),
      child: Container(
        width: 6,
        height: 6,
        decoration: BoxDecoration(color: color, shape: BoxShape.circle),
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
    required this.pendingAttachments,
    required this.capabilities,
    required this.onSend,
    required this.onStop,
    required this.onVoiceRecorded,
    required this.onPickImage,
    required this.onRemoveAttachment,
    required this.onPasteImage,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool isSending;
  final int pendingQueueCount;
  final List<PendingAttachment> pendingAttachments;
  final ServerCapabilities capabilities;
  final VoidCallback onSend;
  final VoidCallback onStop;
  final void Function(Uint8List bytes, String mimeType) onVoiceRecorded;
  final VoidCallback onPickImage;
  final void Function(int index) onRemoveAttachment;
  final void Function(Uint8List bytes) onPasteImage;

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
        // Pending attachment thumbnails.
        if (pendingAttachments.isNotEmpty)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.fromLTRB(12, 8, 12, 0),
            child: SizedBox(
              height: 72,
              child: ListView.separated(
                scrollDirection: Axis.horizontal,
                itemCount: pendingAttachments.length,
                separatorBuilder: (_, _) => const SizedBox(width: 8),
                itemBuilder: (context, index) {
                  final attachment = pendingAttachments[index];
                  return Stack(
                    children: [
                      ClipRRect(
                        borderRadius: BorderRadius.circular(8),
                        child: Image.memory(
                          attachment.bytes,
                          width: 64,
                          height: 64,
                          fit: BoxFit.cover,
                        ),
                      ),
                      Positioned(
                        top: -4,
                        right: -4,
                        child: IconButton(
                          key: Key('remove_attachment_$index'),
                          icon: const Icon(Icons.cancel, size: 18),
                          onPressed: () => onRemoveAttachment(index),
                          padding: EdgeInsets.zero,
                          constraints: const BoxConstraints(),
                          color: Colors.red.shade400,
                        ),
                      ),
                    ],
                  );
                },
              ),
            ),
          ),
        Container(
          padding: EdgeInsets.fromLTRB(12, 8, 12, 12 + bottomInset),
          decoration: const BoxDecoration(
            border: Border(top: BorderSide(color: Colors.black12)),
          ),
          child: Row(
            children: [
              // Image picker button.
              if (!isSending)
                IconButton(
                  key: const Key('attach_image_button'),
                  icon: const Icon(Icons.image_outlined),
                  tooltip: 'Attach image',
                  onPressed: onPickImage,
                ),
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
              if (!isSending) const SizedBox(width: 4),
              Expanded(
                child: KeyboardListener(
                  focusNode: FocusNode(),
                  onKeyEvent: (event) {
                    // Detect Ctrl+V / Cmd+V paste with image data.
                    if (event is KeyDownEvent &&
                        event.logicalKey == LogicalKeyboardKey.keyV &&
                        (HardwareKeyboard.instance.isControlPressed ||
                            HardwareKeyboard.instance.isMetaPressed)) {
                      Clipboard.getData('image/png').then((data) {
                        // ClipboardData doesn't support binary; use
                        // the text field's default paste for text.
                        // Image paste from clipboard is handled via
                        // the super_clipboard package if available.
                      });
                    }
                  },
                  child: TextField(
                    key: const Key('message_input'),
                    controller: controller,
                    focusNode: focusNode,
                    decoration: InputDecoration(
                      hintText: pendingAttachments.isNotEmpty
                          ? 'Add a caption...'
                          : 'Type a message...',
                      border: const OutlineInputBorder(),
                      contentPadding: const EdgeInsets.symmetric(
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
