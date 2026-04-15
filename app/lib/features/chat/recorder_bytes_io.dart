// Native (dart:io) implementation: read recorded file from disk.
import 'dart:io';
import 'dart:typed_data';

/// Reads the recorded audio file at [filePath] and returns the raw bytes
/// together with the known [mimeType] chosen at record-start time.
///
/// Deletion is best-effort: a [FileSystemException] during cleanup does not
/// affect the returned bytes so the voice upload can still proceed.
Future<(Uint8List, String)> readRecordedOutput(
  String filePath,
  String mimeType,
) async {
  final bytes = await File(filePath).readAsBytes();
  try {
    await File(filePath).delete();
  } on FileSystemException {
    // Best-effort cleanup — don't let a deletion failure lose the audio.
  }
  return (bytes, mimeType);
}
