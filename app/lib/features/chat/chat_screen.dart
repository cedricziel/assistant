import 'dart:async';

import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_ui/assistant_ui.dart';
import 'package:desktop_drop/desktop_drop.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:go_router/go_router.dart';

import '../../shared/layout/breakpoints.dart';
import '../../shared/platform/widgets.dart';
import '../../api/api_client.dart';
import '../../api/attachment_service.dart';
import '../../api/capabilities_provider.dart';
import '../../api/connectivity_provider.dart';
import '../../api/models/server_capabilities.dart';
import '../connection/connection_provider.dart';
import '../personas/persona_picker.dart';
import '../personas/personas_provider.dart';
import 'attachment_provider.dart';
import 'chat_announcer.dart';
import 'chat_provider.dart';
import 'command_event_tile.dart';
import 'commands_provider.dart';
import 'conversation_list.dart';
import 'empty_chat.dart';
import 'message_bubble.dart';
import 'image_utils.dart';
import 'queued_message_remove_sheet.dart';
import 'reconnect_banner.dart';
import 'streaming_timeline_entry.dart';
import 'turn_progress_card.dart';
import 'voice_recorder_button.dart';

/// Main chat screen.
///
/// - Left panel: [ConversationListPane] (shown as drawer on narrow screens).
/// - Right panel: message list + input row.
/// - App bar: active persona name + persona picker button.
class ChatScreen extends ConsumerStatefulWidget {
  const ChatScreen({super.key, this.conversationId});

  final String? conversationId;

  @override
  ConsumerState<ChatScreen> createState() => _ChatScreenState();
}

const double _kBottomThreshold = 80.0;

/// Width of the inline conversation list, border included.
const double _kConversationListWidth = 240.0;

class _ChatScreenState extends ConsumerState<ChatScreen> {
  final _inputController = TextEditingController();
  final _scrollController = ScrollController();
  final _inputFocus = FocusNode();
  bool _atBottom = true;
  StreamSubscription<ChatLifecycleEvent>? _lifecycleSub;

  @override
  void initState() {
    super.initState();
    // Turn lifecycle events, never token frames, drive screen-reader
    // announcements; the streaming bubble is not a live region.
    _lifecycleSub = ref
        .read(chatProvider.notifier)
        .lifecycle
        .listen(ref.read(chatAnnouncerProvider).handle);
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
        _loadConversation(focusInput: true);
      });
    }
  }

  void _loadConversation({bool focusInput = false}) {
    final id = widget.conversationId;
    if (id != null) {
      ref.read(chatProvider.notifier).loadConversation(id);
    } else {
      ref.read(chatProvider.notifier).clearConversation();
    }
    _scrollToBottom();
    if (focusInput) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _inputFocus.requestFocus();
      });
    }
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
    unawaited(_lifecycleSub?.cancel());
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
    // Defer focus request: onSubmitted unfocuses the field after the callback
    // returns, so an immediate requestFocus() gets overridden.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _inputFocus.requestFocus();
    });

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
    if (!ref.read(isOnlineProvider)) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('File upload requires an internet connection'),
          ),
        );
      }
      return;
    }

    final files = await FilePicker.pickFiles(
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
    );
    final notifier = ref.read(pendingAttachmentsProvider.notifier);
    for (final file in files) {
      final fileBytes = await file.readAsBytes();
      final mime = mimeFromExtension(file.extension);
      if (isImageMimeType(mime) || mime == 'image/heic') {
        // Image files may need HEIC→PNG conversion.
        final (bytes, filename, normalizedMime) = await normalizeImage(
          fileBytes,
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
        notifier.add(
          PendingAttachment(
            bytes: fileBytes,
            filename: file.name,
            mimeType: mime,
          ),
        );
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
        icon: const Icon(Icons.extension_outlined),
        tooltip: 'Skills',
        onPressed: () => context.go('/skills'),
      ),
    ];

    // Hoisted: `ref.watch` is only valid while building, not inside the
    // LayoutBuilder callback below.
    final pendingAttachments = ref.watch(pendingAttachmentsProvider);
    final commands = ref.watch(commandsProvider).value ?? [];

    // The inline list / drawer decision compares the chat pane's own width
    // (the window minus the nav sidebar), so it sits above the Scaffold,
    // which it drives (body, app bar leading, drawer).
    return LayoutBuilder(
      builder: (context, constraints) {
        final isWide = constraints.maxWidth >= Breakpoints.wide;
        final threadWidth =
            constraints.maxWidth - (isWide ? _kConversationListWidth : 0);
        return Scaffold(
          appBar: AdaptiveNavBar(
            title: activePersonaName,
            leading: isWide
                ? null
                : Builder(
                    builder: (ctx) => IconButton(
                      icon: const AdaptiveIcon(
                        cupertino: CupertinoIcons.line_horizontal_3,
                        material: Icons.menu,
                      ),
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
                    child: ConversationListPane(
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
                    width: _kConversationListWidth,
                    child: Container(
                      decoration: BoxDecoration(
                        border: Border(
                          right: BorderSide(
                            color: Theme.of(context).colorScheme.outlineVariant,
                          ),
                        ),
                      ),
                      child: const ConversationListPane(),
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
                                    color: Theme.of(context)
                                        .colorScheme
                                        .onErrorContainer,
                                    size: 18,
                                  ),
                                  const SizedBox(width: 8),
                                  Expanded(
                                    child: Text(
                                      chatState.error!,
                                      style: TextStyle(
                                        color: Theme.of(context)
                                            .colorScheme
                                            .onErrorContainer,
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
                          child: ThreadDensityScope.fromWidth(
                            width: threadWidth,
                            child: Stack(
                              children: [
                                chatState.messages.isEmpty &&
                                        chatState.pendingQueue.isEmpty
                                    ? const EmptyChat()
                                    : ListView.builder(
                                        controller: _scrollController,
                                        padding: const EdgeInsets.symmetric(
                                          vertical: 16,
                                          horizontal: 12,
                                        ),
                                        itemCount:
                                            chatState.messages.length +
                                            chatState.pendingQueue.length,
                                        itemBuilder: (context, index) {
                                          // Render queued ghost bubbles after the
                                          // committed messages, in send order. The
                                          // actively-streaming message is already in
                                          // `messages` (it was popped from the queue
                                          // when its stream started), so ghost
                                          // bubbles represent only the truly-waiting
                                          // items.
                                          if (index >=
                                              chatState.messages.length) {
                                            final queueIdx =
                                                index -
                                                chatState.messages.length;
                                            final pending = chatState
                                                .pendingQueue[queueIdx];
                                            return QueuedMessageBubble(
                                              message: pending,
                                              onLongPress: () =>
                                                  showQueuedMessageRemoveSheet(
                                                    context,
                                                    ref,
                                                    pending,
                                                    queueIdx,
                                                  ),
                                            );
                                          }

                                          final msg = chatState.messages[index];

                                          // Render command events as compact
                                          // system-event tiles.
                                          if (msg.timelineType ==
                                              TimelineEntryType.command) {
                                            return CommandEventTile(
                                              message: msg,
                                            );
                                          }

                                          // Render non-message timeline entries
                                          // (thinking, tool call, subagent) as
                                          // streaming-aware adaptive sections.
                                          if (msg.timelineType !=
                                              TimelineEntryType.message) {
                                            return StreamingTimelineEntry(
                                              message: msg,
                                              conversationId:
                                                  chatState.conversationId,
                                              entryState: msg.isStale
                                                  ? EntryState.stale
                                                  : msg.isStreaming
                                                  ? EntryState.active
                                                  : EntryState.complete,
                                            );
                                          }

                                          final prevMsg = index > 0
                                              ? chatState.messages[index - 1]
                                              : null;
                                          final isGrouped =
                                              prevMsg != null &&
                                              prevMsg.role == msg.role &&
                                              !prevMsg.isStreaming;
                                          return MessageBubble(
                                            message: msg,
                                            isGrouped: isGrouped,
                                            capabilities: capabilities,
                                            onRetry:
                                                msg.status ==
                                                    MessageStatus.failed
                                                ? () => ref
                                                      .read(
                                                        chatProvider.notifier,
                                                      )
                                                      .retryMessage(msg)
                                                : null,
                                            isStopping: chatState.isStopping,
                                            onStop: () => ref
                                                .read(chatProvider.notifier)
                                                .stopTurn(),
                                            fetchMessageAudio: () {
                                              final api = ref.read(
                                                apiClientProvider,
                                              );
                                              final audioId = msg.audioId;
                                              if (audioId != null) {
                                                return api?.fetchAudio(
                                                      audioId,
                                                    ) ??
                                                    Future.value(null);
                                              }
                                              // On-demand TTS synthesis.
                                              return api?.fetchMessageAudio(
                                                    msg.id,
                                                  ) ??
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
                                      child: const Icon(
                                        Icons.keyboard_arrow_down,
                                      ),
                                    ),
                                  ),
                              ],
                            ),
                          ),
                        ),

                        // Brief banner shown while attemptReconnect is
                        // recovering an interrupted stream after a resume.
                        // Renders nothing on routine foreground transitions.
                        const ReconnectBanner(),

                        // In-flight turn progress card — renders nothing when no
                        // turn is active, an activity label otherwise, a stall
                        // label after 30s of silence.
                        const TurnProgressCard(),

                        // Input row.
                        Composer(
                          controller: _inputController,
                          focusNode: _inputFocus,
                          isSending: chatState.isSending,
                          isStopping: chatState.isStopping,
                          pendingQueueCount: chatState.pendingQueue.length,
                          pendingAttachments: pendingAttachments,
                          commands: commands,
                          showVoice: capabilities.voiceSend,
                          voiceButton: VoiceRecorderButton(
                            onRecordingComplete: _sendVoiceMessage,
                            onError: (err) {
                              ScaffoldMessenger.of(context)
                                  .showSnackBar(SnackBar(content: Text(err)));
                            },
                          ),
                          inputBuilder:
                              (
                                context,
                                controller,
                                focusNode,
                                placeholder,
                                onSubmit,
                              ) => AdaptiveTextField(
                                key: const Key('message_input'),
                                controller: controller,
                                focusNode: focusNode,
                                placeholder: placeholder,
                                cursorColor: Theme.of(context)
                                    .colorScheme
                                    .primary,
                                contentPadding: const EdgeInsets.symmetric(
                                  horizontal: 14,
                                  vertical: 10,
                                ),
                                minLines: 1,
                                maxLines: 6,
                                textInputAction: TextInputAction.send,
                                onSubmitted: (_) => onSubmit(),
                              ),
                          onSend: _sendMessage,
                          onStop: () =>
                              ref.read(chatProvider.notifier).stopTurn(),
                          onPickImage: _pickImages,
                          onRemoveAttachment: (i) => ref
                              .read(pendingAttachmentsProvider.notifier)
                              .removeAt(i),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ), // GestureDetector
        );
      },
    );
  }
}
