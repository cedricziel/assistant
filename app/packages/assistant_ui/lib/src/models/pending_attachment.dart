import 'dart:typed_data';

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
