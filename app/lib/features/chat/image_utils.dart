import 'dart:typed_data';
import 'dart:ui' as ui;

/// Maps a file extension to a MIME type.
///
/// Returns `'application/octet-stream'` for unrecognised extensions.
String mimeFromExtension(String? ext) {
  return switch (ext?.toLowerCase()) {
    'png' => 'image/png',
    'jpg' || 'jpeg' => 'image/jpeg',
    'gif' => 'image/gif',
    'webp' => 'image/webp',
    'heic' || 'heif' => 'image/heic',
    'pdf' => 'application/pdf',
    'txt' => 'text/plain',
    'md' || 'markdown' => 'text/markdown',
    'csv' => 'text/csv',
    'json' => 'application/json',
    _ => 'application/octet-stream',
  };
}

/// MIME types the server accepts for attachment upload.
const supportedMimeTypes = {
  'image/png',
  'image/jpeg',
  'image/gif',
  'image/webp',
  'application/pdf',
  'text/plain',
  'text/markdown',
  'text/csv',
  'application/json',
};

/// Returns `true` if the MIME type is accepted by the server.
bool isSupportedMimeType(String mime) => supportedMimeTypes.contains(mime);

/// Returns `true` if the MIME type is an image type (for thumbnail display).
bool isImageMimeType(String mime) => mime.startsWith('image/');

/// Returns `true` if [ext] is a format that the server does not accept and
/// must be converted client-side (e.g. HEIC from iOS cameras).
bool needsConversion(String? ext) {
  final lower = ext?.toLowerCase();
  return lower == 'heic' || lower == 'heif';
}

/// Converts [bytes] from an unsupported image format (HEIC/HEIF) to PNG using
/// the platform's native image decoder.
///
/// On iOS the platform decoder handles HEIC natively via Image I/O.
/// Returns `null` if the conversion fails (e.g. unsupported platform).
Future<Uint8List?> convertToPng(Uint8List bytes) async {
  try {
    final codec = await ui.instantiateImageCodec(bytes);
    final frame = await codec.getNextFrame();
    final image = frame.image;
    final byteData = await image.toByteData(format: ui.ImageByteFormat.png);
    codec.dispose();
    image.dispose();
    return byteData?.buffer.asUint8List();
  } catch (_) {
    return null;
  }
}

/// Normalises an image picked from the file system.
///
/// If the file is in a format the server does not accept (HEIC/HEIF), it is
/// converted to PNG.  Otherwise the original bytes, filename and MIME type are
/// returned unchanged.
///
/// Returns `(bytes, filename, mimeType)`.
Future<(Uint8List, String, String)> normalizeImage(
  Uint8List bytes,
  String filename,
  String? ext,
) async {
  if (!needsConversion(ext)) {
    return (bytes, filename, mimeFromExtension(ext));
  }

  final pngBytes = await convertToPng(bytes);
  if (pngBytes != null) {
    final newFilename = filename.replaceAll(
      RegExp(r'\.(heic|heif)$', caseSensitive: false),
      '.png',
    );
    return (pngBytes, newFilename, 'image/png');
  }

  // Conversion failed — return original with correct MIME so the server can
  // give a clear rejection message.
  return (bytes, filename, 'image/heic');
}
