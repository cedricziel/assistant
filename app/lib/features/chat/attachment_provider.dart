import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';

/// A pending image attachment selected by the user but not yet uploaded.
class PendingAttachment {
  const PendingAttachment({
    required this.bytes,
    required this.filename,
    required this.mimeType,
  });

  final Uint8List bytes;
  final String filename;
  final String mimeType;
}

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
