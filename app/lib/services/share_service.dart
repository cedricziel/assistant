import 'dart:io';
import 'dart:ui';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:share_plus/share_plus.dart';

/// Platform-adaptive service for sharing and saving content.
///
/// - **Share**: Opens the native share sheet (iOS/macOS) or Web Share API.
/// - **Save**: Opens a file-save dialog (macOS) or uses the share sheet's
///   "Save to Files" (iOS) or triggers a browser download (web).
class ShareService {
  const ShareService._();

  static const instance = ShareService._();

  /// Share plain text via the system share sheet.
  ///
  /// On iPad, [sharePositionOrigin] is required to anchor the popover.
  Future<ShareResult> shareText(String text, {Rect? sharePositionOrigin}) {
    return SharePlus.instance.share(
      ShareParams(text: text, sharePositionOrigin: sharePositionOrigin),
    );
  }

  /// Share a file (from bytes) via the system share sheet.
  Future<ShareResult> shareFile(
    Uint8List bytes, {
    required String filename,
    required String mimeType,
    Rect? sharePositionOrigin,
  }) {
    return SharePlus.instance.share(
      ShareParams(
        files: [XFile.fromData(bytes, name: filename, mimeType: mimeType)],
        sharePositionOrigin: sharePositionOrigin,
      ),
    );
  }

  /// Save text to a user-chosen location.
  ///
  /// - macOS: NSSavePanel via file_picker.
  /// - iOS: Falls back to share sheet (which includes "Save to Files").
  /// - Web: Triggers a blob download.
  Future<bool> saveText(
    String text, {
    required String suggestedFilename,
    Rect? sharePositionOrigin,
  }) async {
    final bytes = Uint8List.fromList(text.codeUnits);
    return saveFile(
      bytes,
      filename: suggestedFilename,
      mimeType: 'text/markdown',
      sharePositionOrigin: sharePositionOrigin,
    );
  }

  /// Save a file (from bytes) to a user-chosen location.
  ///
  /// - macOS: NSSavePanel via file_picker → write to selected path.
  /// - iOS: Share sheet (includes "Save to Files").
  /// - Web: Triggers a blob download via share_plus fallback.
  Future<bool> saveFile(
    Uint8List bytes, {
    required String filename,
    required String mimeType,
    Rect? sharePositionOrigin,
  }) async {
    if (kIsWeb) {
      // On web, share_plus falls back to download when Web Share API
      // is unavailable. Use downloadFallbackEnabled to ensure download.
      await SharePlus.instance.share(
        ShareParams(
          files: [XFile.fromData(bytes, name: filename, mimeType: mimeType)],
          downloadFallbackEnabled: true,
          sharePositionOrigin: sharePositionOrigin,
        ),
      );
      return true;
    }

    if (Platform.isMacOS) {
      return _saveViaPicker(bytes, filename: filename);
    }

    // iOS: Use share sheet which includes "Save to Files".
    await SharePlus.instance.share(
      ShareParams(
        files: [XFile.fromData(bytes, name: filename, mimeType: mimeType)],
        sharePositionOrigin: sharePositionOrigin,
      ),
    );
    return true;
  }

  /// macOS-specific: NSSavePanel → write bytes to the chosen path.
  Future<bool> _saveViaPicker(
    Uint8List bytes, {
    required String filename,
  }) async {
    final path = await FilePicker.saveFile(
      dialogTitle: 'Save As',
      fileName: filename,
    );
    if (path == null) return false;

    final file = File(path);
    await file.writeAsBytes(bytes);
    return true;
  }
}
