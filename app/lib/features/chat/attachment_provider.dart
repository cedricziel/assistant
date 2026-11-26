import 'package:assistant_ui/models.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Manages pending attachments in the chat input area.
class PendingAttachmentsNotifier extends Notifier<List<PendingAttachment>> {
  @override
  List<PendingAttachment> build() => [];

  void add(PendingAttachment attachment) {
    state = [...state, attachment];
  }

  void removeAt(int index) {
    state = [...state]..removeAt(index);
  }

  void clear() {
    state = [];
  }
}

/// Provider for pending attachments in the chat input.
final pendingAttachmentsProvider =
    NotifierProvider<PendingAttachmentsNotifier, List<PendingAttachment>>(
      PendingAttachmentsNotifier.new,
    );
