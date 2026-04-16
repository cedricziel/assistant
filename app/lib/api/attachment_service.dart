import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart';
import 'package:dio/dio.dart';

/// Wraps the generated [AttachmentsApi] for uploading image attachments.
class AttachmentService {
  const AttachmentService(this._api);

  final AttachmentsApi _api;

  /// Upload [bytes] as an image attachment to [conversationId].
  ///
  /// Returns the [AttachmentMetaResponse] on success.
  Future<AttachmentMetaResponse> upload({
    required String conversationId,
    required Uint8List bytes,
    required String filename,
    required String mimeType,
  }) async {
    final file = MultipartFile.fromBytes(
      bytes,
      filename: filename,
      contentType: DioMediaType.parse(mimeType),
    );
    final response = await _api.uploadAttachment(
      id: conversationId,
      file: file,
    );
    return response.data!;
  }
}
