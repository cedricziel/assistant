import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../api/attachment_service.dart';
import '../connection/connection_provider.dart';
import 'attachment_provider.dart';
import 'chat_provider.dart';

/// Uploads the pending attachments, then sends [text] with their ids.
///
/// The screen owns the input controller, focus and the empty-input guard; this
/// keeps the upload conversation-creation race and the send in one place.
Future<void> sendChatMessage(
  WidgetRef ref,
  String text, {
  required ThreadViewportController viewport,
}) async {
  final pending = ref.read(pendingAttachmentsProvider);
  final attachmentIds = <String>[];
  final attachments = <ChatAttachment>[];

  if (pending.isNotEmpty) {
    ref.read(pendingAttachmentsProvider.notifier).clear();
    final api = ref.read(apiClientProvider);
    if (api != null) {
      final chatState = ref.read(chatProvider).value ?? const ChatState();
      var conversationId = chatState.conversationId;
      // Create the conversation if needed (mirrors ChatNotifier.sendMessage).
      if (conversationId == null) {
        final response = await api.conversations.createConversation(
          createConversationRequest: CreateConversationRequest((b) => b),
        );
        final conversation = response.data!;
        conversationId = conversation.id;
        ref
            .read(conversationListProvider.notifier)
            .prependConversation(conversation);
        ref.read(chatProvider.notifier).setConversationId(conversation.id);
      }
      final service = AttachmentService(api.attachments);
      for (final attachment in pending) {
        try {
          final meta = await service.upload(
            conversationId: conversationId,
            bytes: attachment.bytes,
            filename: attachment.filename,
            mimeType: attachment.mimeType,
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
        } catch (error) {
          // Send without the failed attachment rather than dropping the turn.
          debugPrint(
            'Attachment upload failed for ${attachment.filename}: $error',
          );
        }
      }
    }
  }

  // The server rejects empty messages, so use a zero-width space when only
  // attachments are queued. The bubble hides it when attachments exist.
  final message = text.isNotEmpty ? text : '\u200B';
  await ref
      .read(chatProvider.notifier)
      .sendMessage(
        message,
        attachmentIds: attachmentIds,
        attachments: attachments,
      );
  viewport.scrollToBottom();
}
