// Native (dart:io) implementation: read recorded file from disk.
import 'dart:io';
import 'dart:typed_data';

/// Reads the recorded audio file at [filePath], deletes it, and returns the
/// raw bytes together with the known [mimeType] chosen at record-start time.
Future<(Uint8List, String)> readRecordedOutput(
  String filePath,
  String mimeType,
) async {
  final file = File(filePath);
  final bytes = await file.readAsBytes();
  await file.delete();
  return (bytes, mimeType);
}
