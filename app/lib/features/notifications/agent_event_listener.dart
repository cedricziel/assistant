import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../chat/chat_provider.dart';
import 'notification_preferences.dart';
import 'notification_service.dart';

/// Transparent widget that sits above [ChatScreen] and fires local
/// notifications in response to assistant events.
///
/// - New assistant message → "New message" notification (when not `resumed`)
/// - Tool success → "Skill complete" notification
/// - Tool failure → "Skill failed" notification
/// - Chat error → "Assistant error" notification
///
/// All notifications respect the per-category [NotificationPreferences].
/// Lifecycle state is tracked via [WidgetsBindingObserver] so notifications
/// are suppressed when the app is in the foreground.
class AgentEventListener extends ConsumerStatefulWidget {
  const AgentEventListener({super.key, required this.child});

  final Widget child;

  @override
  ConsumerState<AgentEventListener> createState() => _AgentEventListenerState();
}

class _AgentEventListenerState extends ConsumerState<AgentEventListener>
    with WidgetsBindingObserver {
  AppLifecycleState? _lifecycleState;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _lifecycleState = state;
    if (state == AppLifecycleState.resumed) {
      // Silently recover any stream interrupted by iOS backgrounding.
      ref.read(chatProvider.notifier).attemptReconnect();
    }
  }

  bool get _isResumed =>
      _lifecycleState == null || _lifecycleState == AppLifecycleState.resumed;

  @override
  Widget build(BuildContext context) {
    ref.listen<AsyncValue<ChatState>>(chatProvider, _onChatStateChanged);
    return widget.child;
  }

  void _onChatStateChanged(
    AsyncValue<ChatState>? prev,
    AsyncValue<ChatState> next,
  ) {
    final prevState = prev?.value;
    final nextState = next.value;
    if (prevState == null || nextState == null) return;

    final prefs = ref.read(notificationPreferencesProvider).value;
    if (prefs == null) return;

    // New assistant message: streaming finished and last message is assistant.
    if (prevState.isSending &&
        !nextState.isSending &&
        !_isResumed &&
        prefs.notifyChatMessages) {
      final lastMsg = nextState.messages.lastOrNull;
      if (lastMsg != null &&
          lastMsg.isAssistant &&
          lastMsg.content.isNotEmpty) {
        final truncated = lastMsg.content.length > 80
            ? '${lastMsg.content.substring(0, 77)}…'
            : lastMsg.content;
        ref
            .read(notificationServiceProvider)
            .show(
              'New message',
              truncated,
              conversationId: nextState.conversationId,
            );
      }
    }

    // Tool result changed — skill complete or failed notification.
    final prevTool = prevState.lastToolResult;
    final nextTool = nextState.lastToolResult;
    if (nextTool != null && nextTool != prevTool) {
      if (nextTool.isSuccess && prefs.notifySkillComplete) {
        ref
            .read(notificationServiceProvider)
            .show('Skill complete', nextTool.toolName);
      } else if (!nextTool.isSuccess && prefs.notifyAgentErrors) {
        ref
            .read(notificationServiceProvider)
            .show('Skill failed', nextTool.toolName);
      }
    }

    // Agent / chat error.
    if (nextState.error != null &&
        nextState.error != prevState.error &&
        prefs.notifyAgentErrors) {
      final errMsg = nextState.error!;
      final truncated = errMsg.length > 80
          ? '${errMsg.substring(0, 77)}…'
          : errMsg;
      ref.read(notificationServiceProvider).show('Assistant error', truncated);
    }
  }
}
