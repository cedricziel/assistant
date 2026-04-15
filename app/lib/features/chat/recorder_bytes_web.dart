// Web implementation: fetch blob URL and return bytes + actual MIME type.
import 'dart:typed_data';

import 'package:web/web.dart' as web;
import 'dart:js_interop';

/// Reads the recorded output from a blob URL (returned by record.stop() on web).
///
/// Returns the raw bytes and the content-type reported by the browser (e.g.
/// "audio/webm;codecs=opus" on Chrome, "audio/ogg;codecs=opus" on Firefox,
/// "audio/mp4" on Safari).  The MIME is stripped to the base type so the
/// server always receives a clean value.
Future<(Uint8List, String)> readRecordedOutput(
  String blobUrl,
  String fallbackMime,
) async {
  final response = await web.window.fetch(blobUrl.toJS).toDart;
  // content-type may include codec suffix — strip it.
  final rawMime =
      response.headers.get('content-type')?.split(';').first.trim() ??
      fallbackMime;
  final buffer = await response.arrayBuffer().toDart;
  final bytes = buffer.toDart.asUint8List();
  // Revoke the object URL to free memory.
  web.URL.revokeObjectURL(blobUrl);
  return (bytes, rawMime);
}
