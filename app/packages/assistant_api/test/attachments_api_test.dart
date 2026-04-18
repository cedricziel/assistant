import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for AttachmentsApi
void main() {
  final instance = AssistantApi().getAttachmentsApi();

  group(AttachmentsApi, () {
    // `GET /api/attachments/{id}` — serve an attachment, optionally resized.
    //
    // Supports `w` and `h` query params for on-demand image resizing. Resized variants are cached on disk. Responds with `ETag` and `Cache-Control` headers; returns `304 Not Modified` when `If-None-Match` matches.
    //
    //Future serveAttachment(String id, { int w, int h }) async
    test('test serveAttachment', () async {
      // TODO
    });

    // `POST /api/conversations/{id}/attachments` — upload an image attachment.
    //
    // Accepts a `multipart/form-data` body with a single `file` field. Returns `201 Created` with the attachment metadata on success.
    //
    //Future<AttachmentMetaResponse> uploadAttachment(String id, MultipartFile file) async
    test('test uploadAttachment', () async {
      // TODO
    });
  });
}
