import 'package:assistant_api/assistant_api.dart' hide BranchInfo;
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';
import 'chat_history_mapping.dart';
import 'chat_models.dart';

/// Switching the active version of a message, for [ChatNotifier].
///
/// Lives beside the notifier so `chat_provider.dart` does not grow with every
/// branching feature.
mixin ChatBranchSwitching on AsyncNotifier<ChatState> {
  /// Make the version starting at [messageId] the active one.
  ///
  /// Calls `POST /api/conversations/{id}/switch-branch` and replaces the
  /// timeline with the returned detail; the server lands on the most recently
  /// created leaf below [messageId]. [ChatState.isSwitchingBranch] is set
  /// while the request is pending. On failure (409 while a turn is working, a
  /// network error) the messages stay as they are and [ChatState.error] is
  /// set.
  Future<void> switchBranch(String messageId) async {
    final current = state.value ?? const ChatState();
    final conversationId = current.conversationId;
    final api = ref.read(apiClientProvider);
    if (conversationId == null || current.isSwitchingBranch) return;
    if (api == null) {
      state = AsyncData(current.copyWith(error: 'Not connected to server'));
      return;
    }

    state = AsyncData(
      current.copyWith(isSwitchingBranch: true, clearError: true),
    );
    try {
      final response = await api.conversations.switchConversationBranch(
        id: conversationId,
        switchBranchRequest: SwitchBranchRequest(
          (b) => b..messageId = messageId,
        ),
      );
      if (!ref.mounted) return;
      final detail = response.data;
      if (detail == null) throw StateError('Empty switch-branch response');
      final latest = state.value ?? const ChatState();
      if (latest.conversationId != conversationId) return;
      state = AsyncData(
        latest.copyWith(
          messages: chatMessagesFromHistory(
            detail.messages,
            interactions: detail.interactions,
          ),
          isSwitchingBranch: false,
        ),
      );
    } catch (e) {
      if (!ref.mounted) return;
      final latest = state.value ?? const ChatState();
      state = AsyncData(
        latest.copyWith(isSwitchingBranch: false, error: _switchBranchError(e)),
      );
    }
  }
}

String _switchBranchError(Object e) {
  if (e is DioException && e.response?.statusCode == 409) {
    return 'The assistant is still working. Wait for it to finish, then '
        'switch versions.';
  }
  return 'Failed to switch version: $e';
}
