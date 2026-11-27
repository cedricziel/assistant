import 'dart:async';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../api/api_client.dart';
import 'chat_announcer.dart';
import 'chat_provider.dart';

/// Owns the conversation lifecycle [ChatScreen] drives: loading the route's
/// conversation, retrying once the API client appears, the screen-reader
/// announcement subscription and the conversation-open scroll.
///
/// Keeping it separate leaves the screen with provider reads and widget
/// wiring. Callbacks read the providers, so this class needs no `setState`.
class ChatScreenSession {
  ChatScreenSession({
    required this.ref,
    required this.inputFocus,
    required this.viewport,
    required this.isMounted,
  });

  final WidgetRef ref;
  final FocusNode inputFocus;
  final ThreadViewportController viewport;

  /// Whether the screen is still mounted, for callbacks that run after a
  /// frame.
  final bool Function() isMounted;

  StreamSubscription<ChatLifecycleEvent>? _lifecycleSub;

  /// Subscribes to turn lifecycle events. Token frames never drive the
  /// announcements; the streaming bubble is not a live region.
  void start() {
    _lifecycleSub = ref
        .read(chatProvider.notifier)
        .lifecycle
        .listen(ref.read(chatAnnouncerProvider).handle);
  }

  /// Loads [conversationId] into [chatProvider], or clears it when null.
  void load(String? conversationId, {bool focusInput = false}) {
    if (conversationId != null) {
      ref.read(chatProvider.notifier).loadConversation(conversationId);
    } else {
      ref.read(chatProvider.notifier).clearConversation();
    }
    viewport.scrollToBottom();
    if (focusInput) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (isMounted()) inputFocus.requestFocus();
      });
    }
  }

  /// Reloads the conversation once the API client becomes available.
  ///
  /// On a hard reload or deep link the first [load] races the context load
  /// and finds no client, so this retries when the provider transitions
  /// null → non-null or switches clients.
  void onApiClientAvailable(
    ApiClient? previous,
    ApiClient? next,
    String? conversationId,
  ) {
    if (next == null || identical(previous, next) || conversationId == null) {
      return;
    }
    final chatState = ref.read(chatProvider).value;
    if (previous != null || chatState?.conversationId != conversationId) {
      load(conversationId);
    }
  }

  void dispose() {
    unawaited(_lifecycleSub?.cancel());
  }
}
