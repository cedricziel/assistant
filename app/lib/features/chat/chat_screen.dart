import 'dart:async';

import 'package:audioplayers/audioplayers.dart';
import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:cached_network_image/cached_network_image.dart';
import 'package:desktop_drop/desktop_drop.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../shared/platform/platform.dart';
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
import 'command_autocomplete.dart';
import 'command_event_tile.dart';
import 'commands_provider.dart';
import 'conversation_list.dart';
import 'image_utils.dart';
import 'timeline_section.dart';
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
    if (isAppleTouch) HapticFeedback.lightImpact();
    _inputController.clear();
    _inputFocus.requestFocus();

    // Upload pending attachments first, then send with IDs.
    final attachmentIds = <String>[];
    final attachments = <ChatAttachment>[];
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
            attachments.add(
              ChatAttachment(
                id: meta.id,
                filename: meta.filename,
                mimeType: meta.mimeType,
                url: meta.url,
              ),
            );
          } catch (e) {
            // Log but don't block — the message will still be sent without
            // the failed attachment.
            debugPrint('Attachment upload failed for ${p.filename}: $e');
          }
        }
      }
    }

    // Server rejects empty messages, so use a Unicode zero-width space when
    // only images are attached. The bubble hides this when attachments exist.
    final msg = text.isNotEmpty ? text : '\u200B';
    await ref
        .read(chatProvider.notifier)
        .sendMessage(
          msg,
          attachmentIds: attachmentIds,
          attachments: attachments,
        );
    _scrollToBottom();
  }

  Future<void> _sendVoiceMessage(Uint8List bytes, String mimeType) async {
    await ref.read(chatProvider.notifier).sendVoiceMessage(bytes, mimeType);
    _scrollToBottom();
  }

  Future<void> _pickImages() async {
    final result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: [
        'png',
        'jpg',
        'jpeg',
        'gif',
        'webp',
        'heic',
        'heif',
        'pdf',
        'txt',
        'md',
        'csv',
        'json',
      ],
      allowMultiple: true,
      withData: true,
    );
    if (result == null) return;
    final notifier = ref.read(pendingAttachmentsProvider.notifier);
    for (final file in result.files) {
      if (file.bytes != null) {
        final mime = mimeFromExtension(file.extension);
        if (isImageMimeType(mime) || mime == 'image/heic') {
          // Image files may need HEIC→PNG conversion.
          final (bytes, filename, normalizedMime) = await normalizeImage(
            file.bytes!,
            file.name,
            file.extension,
          );
          notifier.add(
            PendingAttachment(
              bytes: bytes,
              filename: filename,
              mimeType: normalizedMime,
            ),
          );
        } else {
          // Non-image files: add directly.
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

    // Watch the active profile reactively so image auth headers update when
    // the profile loads (e.g. after a deep-link hard reload).
    final activeProfile = ref.watch(activeProfileProvider);
    final imageBaseUrl = activeProfile?.baseUrl;
    final imageAuthToken = activeProfile?.token;

    final chatActions = [
      IconButton(
        key: const Key('persona_picker_button'),
        icon: const Icon(Icons.switch_account_outlined),
        tooltip: 'Switch persona',
        onPressed: () => showPersonaPicker(context),
      ),
      IconButton(
        icon: const Icon(Icons.timeline_outlined),
        tooltip: 'Traces',
        onPressed: () => context.go('/traces'),
      ),
      IconButton(
        icon: const Icon(Icons.article_outlined),
        tooltip: 'Logs',
        onPressed: () => context.go('/logs'),
      ),
      IconButton(
        icon: const Icon(Icons.extension_outlined),
        tooltip: 'Skills',
        onPressed: () => context.go('/skills'),
      ),
    ];

    return Scaffold(
      appBar: isAppleTouch
          ? CupertinoNavigationBar(
              middle: Text(activePersonaName),
              leading: isWide
                  ? null
                  : Builder(
                      builder: (ctx) => GestureDetector(
                        onTap: () => Scaffold.of(ctx).openDrawer(),
                        child: const Icon(CupertinoIcons.line_horizontal_3),
                      ),
                    ),
              trailing: Row(
                mainAxisSize: MainAxisSize.min,
                children: chatActions,
              ),
            )
          : AppBar(
              title: Text(activePersonaName),
              leading: isWide
                  ? null
                  : Builder(
                      builder: (ctx) => IconButton(
                        icon: const Icon(Icons.menu),
                        onPressed: () => Scaffold.of(ctx).openDrawer(),
                      ),
                    ),
              actions: chatActions,
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
                  decoration: BoxDecoration(
                    border: Border(
                      right: BorderSide(
                        color: Theme.of(context).colorScheme.outlineVariant,
                      ),
                    ),
                  ),
                  child: const ConversationList(),
                ),
              ),

            // Chat area with drop target for file attachments.
            Expanded(
              child: DropTarget(
                onDragDone: (details) {
                  final notifier = ref.read(
                    pendingAttachmentsProvider.notifier,
                  );
                  for (final file in details.files) {
                    file.readAsBytes().then((bytes) {
                      final ext = file.name.split('.').last;
                      final mime = mimeFromExtension(ext);
                      if (isSupportedMimeType(mime)) {
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
                        color: Theme.of(context).colorScheme.errorContainer,
                        child: Padding(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 16,
                            vertical: 8,
                          ),
                          child: Row(
                            children: [
                              Icon(
                                Icons.warning_amber_outlined,
                                color: Theme.of(
                                  context,
                                ).colorScheme.onErrorContainer,
                                size: 18,
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Text(
                                  chatState.error!,
                                  style: TextStyle(
                                    color: Theme.of(
                                      context,
                                    ).colorScheme.onErrorContainer,
                                  ),
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

                                    // Render command events as compact
                                    // system-event tiles.
                                    if (msg.timelineType ==
                                        TimelineEntryType.command) {
                                      return CommandEventTile(message: msg);
                                    }

                                    // Render non-message timeline entries
                                    // (thinking, tool call, subagent) as
                                    // compact expandable sections.
                                    if (msg.timelineType !=
                                        TimelineEntryType.message) {
                                      return ChatTimelineSection(message: msg);
                                    }

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
                                      imageBaseUrl: imageBaseUrl,
                                      imageAuthToken: imageAuthToken,
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
                      commands: ref.watch(commandsProvider).value ?? [],
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
  final Future<({Uint8List bytes, String mimeType})?> Function()
  fetchMessageAudio;
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
                  ? Border.all(color: colorScheme.error, width: 1.5)
                  : null,
            ),
            child:
                message.isStreaming &&
                    message.content.isEmpty &&
                    message.toolCalls.isEmpty &&
                    message.tokenStream == null
                ? _streamingDotsIndicator()
                : isUser
                ? Column(
                    crossAxisAlignment: CrossAxisAlignment.end,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (message.attachments.isNotEmpty)
                        _attachmentThumbnails(context),
                      // Voice message: show audio player + collapsible transcript.
                      if (message.audioBytes != null)
                        _VoiceMessagePlayer(
                          audioBytes: message.audioBytes!,
                          audioMimeType: message.audioMimeType,
                          transcript: message.content,
                          foregroundColor: colorScheme.onPrimary,
                        )
                      // Hide text row when content is only whitespace /
                      // zero-width space and attachments provide the visual.
                      else if (message.content.trim().isNotEmpty)
                        Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            if (isFailed)
                              Padding(
                                padding: const EdgeInsets.only(right: 6),
                                child: Icon(
                                  Icons.error_outline,
                                  size: 16,
                                  color: colorScheme.error,
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
          // Non-image attachments: render as file tile with icon.
          if (!isImageMimeType(att.mimeType)) {
            return Container(
              width: 150,
              height: 150,
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerLowest,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(
                    iconForMime(att.mimeType),
                    color: colorScheme.outline,
                    size: 32,
                  ),
                  const SizedBox(height: 6),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 8),
                    child: Text(
                      att.filename,
                      style: TextStyle(
                        fontSize: 10,
                        color: colorScheme.outline,
                      ),
                      overflow: TextOverflow.ellipsis,
                      textAlign: TextAlign.center,
                    ),
                  ),
                ],
              ),
            );
          }

          // Without a base URL the relative path cannot be resolved.
          if (imageBaseUrl == null) {
            return Container(
              width: 150,
              height: 150,
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerLowest,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(Icons.image, color: colorScheme.outline),
                  const SizedBox(height: 4),
                  Text(
                    att.filename,
                    style: TextStyle(fontSize: 10, color: colorScheme.outline),
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            );
          }
          final thumbUrl = '$imageBaseUrl${att.url}?w=300';
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
                      child: CircularProgressIndicator.adaptive(strokeWidth: 2),
                    ),
                  ),
                ),
                errorWidget: (_, url, _) => GestureDetector(
                  onTap: () {
                    CachedNetworkImage.evictFromCache(url);
                    // Trigger a rebuild to retry loading.
                    (context as Element).markNeedsBuild();
                  },
                  child: Container(
                    width: 150,
                    height: 150,
                    color: colorScheme.surfaceContainerLowest,
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(Icons.refresh, color: colorScheme.outline),
                        const SizedBox(height: 4),
                        Text(
                          'Tap to retry',
                          style: TextStyle(
                            fontSize: 10,
                            color: colorScheme.outline,
                          ),
                        ),
                      ],
                    ),
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
    if (imageBaseUrl == null) return;
    final fullUrl = '$imageBaseUrl${attachment.url}?w=1920';
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
                    const Center(child: CircularProgressIndicator.adaptive()),
                errorWidget: (_, _, _) => Center(
                  child: Icon(
                    Icons.broken_image,
                    size: 64,
                    color: Theme.of(
                      context,
                    ).colorScheme.onSurface.withValues(alpha: 0.7),
                  ),
                ),
              ),
            ),
            IconButton(
              icon: Icon(
                Icons.close,
                color: Theme.of(context).colorScheme.onPrimary,
              ),
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
  final Future<({Uint8List bytes, String mimeType})?> Function()
  fetchMessageAudio;
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
          color: colorScheme.error,
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

  final Future<({Uint8List bytes, String mimeType})?> Function() fetchAudio;
  final Color color;

  @override
  State<_ReadAloudAction> createState() => _ReadAloudActionState();
}

class _ReadAloudActionState extends State<_ReadAloudAction> {
  final _player = AudioPlayer();
  _ReadAloudState _state = _ReadAloudState.idle;
  ({Uint8List bytes, String mimeType})? _cachedAudio;
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
          _cachedAudio ??= await widget.fetchAudio();
          final audio = _cachedAudio;
          if (audio == null || !mounted) {
            _showError();
            return;
          }
          await _player.play(
            BytesSource(audio.bytes, mimeType: audio.mimeType),
          );
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
                  child: CircularProgressIndicator.adaptive(
                    strokeWidth: 2,
                    valueColor: AlwaysStoppedAnimation<Color>(widget.color),
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
    final colorScheme = Theme.of(context).colorScheme;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.chat_bubble_outline,
            size: 64,
            color: colorScheme.outlineVariant,
          ),
          const SizedBox(height: 16),
          Text(
            'Start a conversation',
            style: TextStyle(
              fontSize: 18,
              color: colorScheme.onSurfaceVariant,
              fontWeight: FontWeight.w500,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Type a message below to begin.',
            style: TextStyle(color: colorScheme.onSurfaceVariant),
          ),
        ],
      ),
    );
  }
}

// -- Input row ---------------------------------------------------------------

class _InputRow extends StatefulWidget {
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
    this.commands = const [],
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
  final List<CommandEntry> commands;

  @override
  State<_InputRow> createState() => _InputRowState();
}

class _InputRowState extends State<_InputRow> {
  bool _showAutocomplete = false;
  String _commandFilter = '';

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_onTextChanged);
  }

  @override
  void didUpdateWidget(covariant _InputRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller.removeListener(_onTextChanged);
      widget.controller.addListener(_onTextChanged);
    }
  }

  @override
  void dispose() {
    widget.controller.removeListener(_onTextChanged);
    super.dispose();
  }

  void _onTextChanged() {
    final text = widget.controller.text;
    final shouldShow = text.startsWith('/') && widget.commands.isNotEmpty;
    final filter = shouldShow ? text.substring(1).split(' ').first : '';

    if (shouldShow != _showAutocomplete || filter != _commandFilter) {
      setState(() {
        _showAutocomplete = shouldShow;
        _commandFilter = filter;
      });
    }
  }

  void _onCommandSelected(CommandEntry cmd) {
    if (cmd.hasArgs) {
      // Fill the input with the command and a trailing space for the arg.
      widget.controller.text = '/${cmd.name} ';
      widget.controller.selection = TextSelection.fromPosition(
        TextPosition(offset: widget.controller.text.length),
      );
    } else {
      // No-arg command: set text and submit through normal send path.
      widget.controller.text = '/${cmd.name}';
      setState(() => _showAutocomplete = false);
      widget.onSend();
    }
  }

  @override
  Widget build(BuildContext context) {
    final bottomInset = MediaQuery.of(context).padding.bottom;
    final colorScheme = Theme.of(context).colorScheme;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Command autocomplete popup — shown when text starts with /.
        if (_showAutocomplete)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: CommandAutocompletePopup(
              key: const Key('command_autocomplete_popup'),
              commands: widget.commands,
              filter: _commandFilter,
              onSelect: _onCommandSelected,
            ),
          ),
        // Queue depth badge — visible when messages are waiting.
        if (widget.pendingQueueCount > 0)
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
                  '${widget.pendingQueueCount} message${widget.pendingQueueCount == 1 ? '' : 's'} queued',
                  key: const Key('queue_depth_badge'),
                  style: TextStyle(fontSize: 12, color: Colors.amber.shade800),
                ),
              ],
            ),
          ),
        // Pending attachment thumbnails.
        if (widget.pendingAttachments.isNotEmpty)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.fromLTRB(12, 8, 12, 0),
            child: SizedBox(
              height: 72,
              child: ListView.separated(
                scrollDirection: Axis.horizontal,
                itemCount: widget.pendingAttachments.length,
                separatorBuilder: (_, _) => const SizedBox(width: 8),
                itemBuilder: (context, index) {
                  final attachment = widget.pendingAttachments[index];
                  return Stack(
                    children: [
                      ClipRRect(
                        borderRadius: BorderRadius.circular(8),
                        child: isImageMimeType(attachment.mimeType)
                            ? Image.memory(
                                attachment.bytes,
                                width: 64,
                                height: 64,
                                fit: BoxFit.cover,
                              )
                            : Container(
                                width: 64,
                                height: 64,
                                color: colorScheme.surfaceContainerHighest,
                                child: Column(
                                  mainAxisAlignment: MainAxisAlignment.center,
                                  children: [
                                    Icon(
                                      iconForMime(attachment.mimeType),
                                      size: 28,
                                      color: colorScheme.onSurfaceVariant,
                                    ),
                                    const SizedBox(height: 2),
                                    Text(
                                      attachment.filename.length > 8
                                          ? '${attachment.filename.substring(0, 8)}…'
                                          : attachment.filename,
                                      style: Theme.of(context)
                                          .textTheme
                                          .labelSmall
                                          ?.copyWith(
                                            color: colorScheme.onSurfaceVariant,
                                          ),
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ],
                                ),
                              ),
                      ),
                      Positioned(
                        top: -4,
                        right: -4,
                        child: IconButton(
                          key: Key('remove_attachment_$index'),
                          icon: const Icon(Icons.cancel, size: 18),
                          onPressed: () => widget.onRemoveAttachment(index),
                          padding: EdgeInsets.zero,
                          constraints: const BoxConstraints(),
                          color: colorScheme.error,
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
          decoration: BoxDecoration(
            border: Border(top: BorderSide(color: colorScheme.outlineVariant)),
          ),
          child: Row(
            children: [
              // Image picker button.
              if (!widget.isSending)
                IconButton(
                  key: const Key('attach_image_button'),
                  icon: const Icon(Icons.image_outlined),
                  tooltip: 'Attach image',
                  onPressed: widget.onPickImage,
                ),
              // Voice recorder button — shown when server supports voice send.
              if (widget.capabilities.voiceSend && !widget.isSending)
                VoiceRecorderButton(
                  onRecordingComplete: widget.onVoiceRecorded,
                  onError: (err) {
                    ScaffoldMessenger.of(
                      context,
                    ).showSnackBar(SnackBar(content: Text(err)));
                  },
                ),
              if (!widget.isSending) const SizedBox(width: 4),
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
                  child: isAppleTouch
                      ? CupertinoTextField(
                          key: const Key('message_input'),
                          controller: widget.controller,
                          focusNode: widget.focusNode,
                          placeholder: widget.pendingAttachments.isNotEmpty
                              ? 'Add a caption...'
                              : 'Type a message...',
                          padding: const EdgeInsets.symmetric(
                            horizontal: 14,
                            vertical: 10,
                          ),
                          minLines: 1,
                          maxLines: 6,
                          textInputAction: TextInputAction.send,
                          onSubmitted: (_) => widget.onSend(),
                        )
                      : TextField(
                          key: const Key('message_input'),
                          controller: widget.controller,
                          focusNode: widget.focusNode,
                          decoration: InputDecoration(
                            hintText: widget.pendingAttachments.isNotEmpty
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
                          onSubmitted: (_) => widget.onSend(),
                        ),
                ),
              ),
              const SizedBox(width: 8),
              if (widget.isSending)
                IconButton.filled(
                  key: const Key('stop_button'),
                  onPressed: widget.onStop,
                  icon: const Icon(Icons.stop_rounded),
                )
              else
                IconButton.filled(
                  key: const Key('send_button'),
                  onPressed: widget.onSend,
                  icon: const Icon(Icons.send),
                ),
            ],
          ),
        ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// Voice message player — play/pause + progress bar + collapsible transcript
// ---------------------------------------------------------------------------

class _VoiceMessagePlayer extends StatefulWidget {
  const _VoiceMessagePlayer({
    required this.audioBytes,
    required this.transcript,
    required this.foregroundColor,
    this.audioMimeType,
  });

  final Uint8List audioBytes;
  final String transcript;
  final Color foregroundColor;
  final String? audioMimeType;

  @override
  State<_VoiceMessagePlayer> createState() => _VoiceMessagePlayerState();
}

class _VoiceMessagePlayerState extends State<_VoiceMessagePlayer> {
  final _player = AudioPlayer();
  bool _isPlaying = false;
  Duration _position = Duration.zero;
  Duration _duration = Duration.zero;
  bool _transcriptExpanded = false;

  @override
  void initState() {
    super.initState();
    _player.onPlayerComplete.listen((_) {
      if (mounted) {
        setState(() {
          _isPlaying = false;
          _position = Duration.zero;
        });
      }
    });
    _player.onPositionChanged.listen((p) {
      if (mounted) setState(() => _position = p);
    });
    _player.onDurationChanged.listen((d) {
      if (mounted) setState(() => _duration = d);
    });
  }

  @override
  void dispose() {
    _player.dispose();
    super.dispose();
  }

  Future<void> _toggle() async {
    if (_isPlaying) {
      await _player.pause();
      setState(() => _isPlaying = false);
    } else {
      if (_position == Duration.zero) {
        await _player.play(
          BytesSource(widget.audioBytes, mimeType: widget.audioMimeType),
        );
      } else {
        await _player.resume();
      }
      setState(() => _isPlaying = true);
    }
  }

  String _formatDuration(Duration d) {
    final minutes = d.inMinutes;
    final seconds = d.inSeconds % 60;
    return '$minutes:${seconds.toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    final fg = widget.foregroundColor;
    final hasTranscript =
        widget.transcript.trim().isNotEmpty &&
        widget.transcript.trim() != '🎤 Voice message';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        // Player row: play/pause + progress + duration.
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            GestureDetector(
              onTap: _toggle,
              child: Icon(
                _isPlaying
                    ? Icons.pause_circle_filled
                    : Icons.play_circle_filled,
                color: fg,
                size: 28,
              ),
            ),
            const SizedBox(width: 8),
            SizedBox(
              width: 120,
              child: SliderTheme(
                data: SliderThemeData(
                  trackHeight: 3,
                  thumbShape: const RoundSliderThumbShape(
                    enabledThumbRadius: 5,
                  ),
                  activeTrackColor: fg,
                  inactiveTrackColor: fg.withValues(alpha: 0.3),
                  thumbColor: fg,
                  overlayShape: SliderComponentShape.noOverlay,
                ),
                child: Slider(
                  value: _duration.inMilliseconds > 0
                      ? _position.inMilliseconds / _duration.inMilliseconds
                      : 0,
                  onChanged: (v) {
                    final target = Duration(
                      milliseconds: (v * _duration.inMilliseconds).round(),
                    );
                    _player.seek(target);
                  },
                ),
              ),
            ),
            const SizedBox(width: 6),
            Text(
              _formatDuration(
                _isPlaying || _position > Duration.zero ? _position : _duration,
              ),
              style: TextStyle(fontSize: 11, color: fg),
            ),
          ],
        ),
        // Collapsible transcript.
        if (hasTranscript) ...[
          const SizedBox(height: 4),
          GestureDetector(
            onTap: () =>
                setState(() => _transcriptExpanded = !_transcriptExpanded),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  _transcriptExpanded ? Icons.expand_less : Icons.expand_more,
                  size: 16,
                  color: fg.withValues(alpha: 0.7),
                ),
                const SizedBox(width: 4),
                Flexible(
                  child: Text(
                    _transcriptExpanded
                        ? widget.transcript
                        : _truncate(widget.transcript, 40),
                    style: TextStyle(
                      fontSize: 12,
                      color: fg.withValues(alpha: 0.7),
                      fontStyle: FontStyle.italic,
                    ),
                    maxLines: _transcriptExpanded ? null : 1,
                    overflow: _transcriptExpanded
                        ? TextOverflow.visible
                        : TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
          ),
        ],
      ],
    );
  }

  static String _truncate(String text, int maxLen) {
    if (text.length <= maxLen) return text;
    return '${text.substring(0, maxLen)}\u2026';
  }
}
