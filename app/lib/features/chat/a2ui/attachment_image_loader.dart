import 'dart:typed_data';

import 'package:dio/dio.dart';

/// Loads attachment bytes for surface `Image` components.
///
/// `GET /api/attachments/{id}` needs the bearer token and the space header, so
/// the stock network `Image` cannot fetch it. The seam lets tests assert the
/// headers without a server.
abstract interface class AttachmentImageLoader {
  Future<Uint8List> load(String attachmentId);
}

/// Loads attachments through the app's authenticated [Dio], the one that
/// carries the bearer interceptor and the `X-Space-Id` interceptor.
class DioAttachmentImageLoader implements AttachmentImageLoader {
  const DioAttachmentImageLoader(this._dio);

  final Dio _dio;

  @override
  Future<Uint8List> load(String attachmentId) async {
    final response = await _dio.get<List<int>>(
      '/api/attachments/$attachmentId',
      options: Options(responseType: ResponseType.bytes),
    );
    return Uint8List.fromList(response.data ?? const []);
  }
}
