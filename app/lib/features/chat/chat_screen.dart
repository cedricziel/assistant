import 'package:assistant_ui/assistant_ui.dart';
import 'package:desktop_drop/desktop_drop.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:go_router/go_router.dart';

import '../../shared/layout/breakpoints.dart';
import '../../shared/platform/widgets.dart';
import '../../api/capabilities_provider.dart';
import '../../api/models/server_capabilities.dart';
import '../connection/connection_provider.dart';
import '../personas/persona_picker.dart';
import '../personas/personas_provider.dart';
import '../settings/response_timing_preference.dart';
import 'attachment_provider.dart';
import 'chat_announcer.dart';
import 'chat_attachment_picker.dart';
import 'chat_markdown_config.dart';
import 'chat_provider.dart';
import 'chat_screen_session.dart';
import 'chat_send_message.dart';
import 'chat_thread_pane.dart';
import 'commands_provider.dart';
import 'conversation_list.dart';
import 'message_feedback_provider.dart';
import 'queued_message_remove_sheet.dart';
import 'voice_recorder_button.dart';

/// Feature flag for the stall-state "Skip" button that triggers
/// `POST /api/conversations/{c}/turns/{t}/cancel`.
///
/// Default `true` — the cancel endpoint is live (PR #853). Flip to
/// `false` as a kill switch if the cancel path misbehaves in the wild;
/// the visual stall card remains regardless, only the affordance to
/// cancel from the UI is gated. Per
/// `openspec/changes/turn-status-endpoint/design.md` open question 1
/// (gate "for one release while we measure cancel reliability").
const bool kSkipButtonEnabled = true;

/// Main chat screen.
///
/// - Left panel: [ConversationListPane] (shown as drawer on narrow screens).
/// - Right panel: [ChatThreadPane] with the thread and the composer.
/// - App bar: active persona name + persona picker button.
class ChatScreen extends ConsumerStatefulWidget {
  const ChatScreen({super.key, this.conversationId});

  final String? conversationId;

  @override
  ConsumerState<ChatScreen> createState() => _ChatScreenState();
}

/// Width of the inline conversation list, border included.
const double _kConversationListWidth = 240.0;

class _ChatScreenState extends ConsumerState<ChatScreen> {
  final _inputController = TextEditingController();
  final _threadController = ThreadViewportController();
  final _inputFocus = FocusNode();

  /// Resolves top-level tool call bodies to their structured result views.
  /// Built once so the scope's resolver identity stays stable across builds;
  /// subagent timelines never consult it.
  final _toolResultRegistry = ToolResultRendererRegistry(
    byKind: defaultToolResultRenderers(),
  );

  /// Bumped on every `chatProvider` emission so [ThreadViewport] follows
  /// in-place mutations (thinking tokens, tool status flips) that change no
  /// item count.
  int _revision = 0;
  late final ChatScreenSession _session;

  @override
  void initState() {
    super.initState();
    _session = ChatScreenSession(
      ref: ref,
      inputFocus: _inputFocus,
      viewport: _threadController,
      isMounted: () => mounted,
    )..start();
    _inputFocus.addListener(_onInputFocusChange);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _session.load(widget.conversationId);
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
      if (mounted) _threadController.scrollToBottomIfNear();
    });
  }

  @override
  void didUpdateWidget(ChatScreen old) {
    super.didUpdateWidget(old);
    if (old.conversationId != widget.conversationId) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _session.load(widget.conversationId, focusInput: true);
      });
    }
  }

  @override
  void dispose() {
    _session.dispose();
    _inputFocus.removeListener(_onInputFocusChange);
    _inputController.dispose();
    _inputFocus.dispose();
    super.dispose();
  }

  /// Copies [code] to the clipboard and announces it, so the result of the
  /// markdown code block's copy button is also heard.
  Future<void> _copyCode(String code) async {
    await Clipboard.setData(ClipboardData(text: code));
    if (!mounted) return;
    ref.read(chatAnnouncerProvider).say('Code copied');
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
    await sendChatMessage(ref, text, viewport: _threadController);
  }

  Future<void> _sendVoiceMessage(Uint8List bytes, String mimeType) async {
    await ref.read(chatProvider.notifier).sendVoiceMessage(bytes, mimeType);
    _threadController.scrollToBottom();
  }

  Future<void> _pickImages() => pickChatAttachments(ref, context);

  /// Shows a failed feedback vote as a snackbar rather than the persistent
  /// thread error banner: the selection was already rolled back and the
  /// message is transient.
  void _showFeedbackError(String? previous, String? next) {
    if (next == null || next == previous || next != kFeedbackSaveErrorMessage) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(next)));
      ref.read(chatProvider.notifier).dismissError();
    });
  }

  @override
  Widget build(BuildContext context) {
    // Retry loading the conversation once the API client becomes available.
    // Handles deep-link / hard-reload races where the active context loads
    // after the first _loadConversation() call.
    ref.listen(
      apiClientProvider,
      (previous, next) =>
          _session.onApiClientAvailable(previous, next, widget.conversationId),
    );

    final chatAsync = ref.watch(chatProvider);
    final chatState = chatAsync.value ?? const ChatState();
    // Thumbs, pending sends and the comment field are per-message feedback
    // state, independent of the conversation list.
    final feedbackState = ref.watch(messageFeedbackControllerProvider);
    // Tell the thread viewport that something changed; it follows only if the
    // user is at the bottom.
    ref.listen(chatProvider, (previous, next) {
      _revision++;
      _showFeedbackError(previous?.value?.error, next.value?.error);
    });

    final personasAsync = ref.watch(personasProvider);
    final activePersonaName =
        personasAsync.value?.activePersona?.name ?? 'Assistant';

    // Resolve capabilities only when an API client is available (avoids
    // triggering async work in tests that have no active profile).
    final hasClient = ref.watch(apiClientProvider) != null;
    final capabilities = hasClient
        ? (ref.watch(capabilitiesProvider).value ?? ServerCapabilities.disabled)
        : ServerCapabilities.disabled;

    // Watch the active profile reactively so image auth headers update when
    // the profile loads (e.g. after a deep-link hard reload).
    final activeProfile = ref.watch(activeProfileProvider);

    // Hoisted: `ref.watch` is only valid while building, not inside the
    // LayoutBuilder callback below.
    final pendingAttachments = ref.watch(pendingAttachmentsProvider);
    final commands = ref.watch(commandsProvider).value ?? [];
    final showResponseTiming =
        ref.watch(showResponseTimingProvider).value ?? false;

    // The inline list / drawer decision compares the chat pane's own width
    // (the window minus the nav sidebar), so it sits above the Scaffold,
    // which it drives (body, app bar leading, drawer).
    final pane = LayoutBuilder(
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
                      tooltip: 'Open conversations',
                      icon: const AdaptiveIcon(
                        cupertino: CupertinoIcons.line_horizontal_3,
                        material: Icons.menu,
                      ),
                      onPressed: () => Scaffold.of(ctx).openDrawer(),
                    ),
                  ),
            actions: [
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
            ],
          ),
          drawer: isWide
              ? null
              : Drawer(
                  child: SafeArea(
                    child: ConversationListPane(
                      selectedId: widget.conversationId,
                      onConversationSelected: () => Navigator.of(context).pop(),
                    ),
                  ),
                ),
          body: GestureDetector(
            onTap: () => FocusScope.of(context).unfocus(),
            child: Row(
              children: [
                if (isWide)
                  SizedBox(
                    width: _kConversationListWidth,
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        border: Border(
                          right: BorderSide(
                            color: Theme.of(context).colorScheme.outlineVariant,
                          ),
                        ),
                      ),
                      child: ConversationListPane(
                        selectedId: widget.conversationId,
                      ),
                    ),
                  ),
                // Chat area with drop target for file attachments.
                Expanded(
                  child: DropTarget(
                    onDragDone: (details) =>
                        addDroppedAttachments(ref, details.files),
                    child: ChatThreadPane(
                      chatState: chatState,
                      banners: [
                        if (chatState.error != null &&
                            chatState.error != kFeedbackSaveErrorMessage)
                          ThreadErrorBanner(
                            message: chatState.error!,
                            onDismiss: ref
                                .read(chatProvider.notifier)
                                .dismissError,
                          ),
                        if (chatState.isLoadingHistory)
                          const LinearProgressIndicator(),
                      ],
                      threadWidth: threadWidth,
                      threadController: _threadController,
                      revision: _revision,
                      capabilities: capabilities,
                      pendingAttachments: pendingAttachments,
                      commands: commands,
                      inputController: _inputController,
                      inputFocus: _inputFocus,
                      activePersonaName: activePersonaName,
                      showResponseTiming: showResponseTiming,
                      feedbackState: feedbackState,
                      onFeedbackChanged: (message, rating) => ref
                          .read(chatProvider.notifier)
                          .setMessageRating(message.id, rating),
                      onSubmitFeedbackComment: (message, comment) => ref
                          .read(chatProvider.notifier)
                          .submitMessageFeedbackComment(message.id, comment),
                      onDismissFeedbackComment: (message) => ref
                          .read(chatProvider.notifier)
                          .dismissMessageFeedbackComment(message.id),
                      showSkip: kSkipButtonEnabled,
                      showVoice: capabilities.voiceSend,
                      voiceButton: VoiceRecorderButton(
                        onRecordingComplete: _sendVoiceMessage,
                        onError: (error) {
                          ScaffoldMessenger.of(context)
                              .showSnackBar(SnackBar(content: Text(error)));
                        },
                      ),
                      imageBaseUrl: activeProfile?.baseUrl,
                      imageAuthToken: activeProfile?.token,
                      onSend: _sendMessage,
                      onStop: ref.read(chatProvider.notifier).stopTurn,
                      onPickImage: _pickImages,
                      onRemoveAttachment: (index) => ref
                          .read(pendingAttachmentsProvider.notifier)
                          .removeAt(index),
                      onShowQueuedMessage: (message, index) =>
                          showQueuedMessageRemoveSheet(
                            context,
                            ref,
                            message,
                            index,
                          ),
                      onInteractionAnswer: (id, answer) =>
                          ref.read(chatProvider.notifier).respond(id, answer),
                      onRetry: (message) =>
                          ref.read(chatProvider.notifier).retryMessage(message),
                      fetchMessageAudio: (message) {
                        final api = ref.read(apiClientProvider);
                        final audioId = message.audioId;
                        if (audioId != null) {
                          return api?.fetchAudio(audioId) ?? Future.value(null);
                        }
                        // On-demand TTS synthesis.
                        return api?.fetchMessageAudio(message.id) ??
                            Future.value(null);
                      },
                    ),
                  ),
                ),
              ],
            ),
          ), // GestureDetector
        );
      },
    );
    return ChatMarkdownConfig(
      onCopyCode: _copyCode,
      child: ToolResultRendererScope(
        resolve: _toolResultRegistry.build,
        child: pane,
      ),
    );
  }
}
