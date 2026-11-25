import 'dart:async';

import 'package:flutter/services.dart';

import '../../shared/platform/widgets.dart';
import '../../services/share_service.dart';

/// Context-menu actions (save, share) for a user voice message.
List<AdaptiveContextMenuAction> voiceMessageActions(
  BuildContext context,
  Uint8List audioBytes,
  String mimeType,
) {
  final ext = mimeType.contains('mp4') ? 'mp4' : 'webm';
  final filename =
      'voice-message-${DateTime.now().toIso8601String().split('T').first}.$ext';
  return [
    AdaptiveContextMenuAction(
      label: 'Save Audio',
      cupertinoIcon: CupertinoIcons.square_arrow_down,
      materialIcon: Icons.save_alt,
      onPressed: () async {
        final box = context.findRenderObject() as RenderBox?;
        final origin = box != null
            ? box.localToGlobal(Offset.zero) & box.size
            : null;
        await ShareService.instance.saveFile(
          audioBytes,
          filename: filename,
          mimeType: mimeType,
          sharePositionOrigin: origin,
        );
      },
    ),
    AdaptiveContextMenuAction(
      label: 'Share',
      cupertinoIcon: CupertinoIcons.share,
      materialIcon: Icons.share,
      onPressed: () async {
        final box = context.findRenderObject() as RenderBox?;
        final origin = box != null
            ? box.localToGlobal(Offset.zero) & box.size
            : null;
        await ShareService.instance.shareFile(
          audioBytes,
          filename: filename,
          mimeType: mimeType,
          sharePositionOrigin: origin,
        );
      },
    ),
  ];
}

/// Context-menu actions (save, share) for an assistant message's audio,
/// fetched lazily through [fetchAudio].
List<AdaptiveContextMenuAction> ttsAudioActions(
  BuildContext context,
  Future<({Uint8List bytes, String mimeType})?> Function() fetchAudio,
) {
  return [
    AdaptiveContextMenuAction(
      label: 'Save Audio',
      cupertinoIcon: CupertinoIcons.square_arrow_down,
      materialIcon: Icons.save_alt,
      onPressed: () async {
        final box = context.findRenderObject() as RenderBox?;
        final origin = box != null
            ? box.localToGlobal(Offset.zero) & box.size
            : null;
        final audio = await fetchAudio();
        if (audio == null) return;
        final ext = audio.mimeType.contains('mp4') ? 'mp4' : 'webm';
        final filename =
            'assistant-audio-${DateTime.now().toIso8601String().split('T').first}.$ext';
        await ShareService.instance.saveFile(
          audio.bytes,
          filename: filename,
          mimeType: audio.mimeType,
          sharePositionOrigin: origin,
        );
      },
    ),
    AdaptiveContextMenuAction(
      label: 'Share',
      cupertinoIcon: CupertinoIcons.share,
      materialIcon: Icons.share,
      onPressed: () async {
        final box = context.findRenderObject() as RenderBox?;
        final origin = box != null
            ? box.localToGlobal(Offset.zero) & box.size
            : null;
        final audio = await fetchAudio();
        if (audio == null) return;
        final ext = audio.mimeType.contains('mp4') ? 'mp4' : 'webm';
        final filename =
            'assistant-audio-${DateTime.now().toIso8601String().split('T').first}.$ext';
        await ShareService.instance.shareFile(
          audio.bytes,
          filename: filename,
          mimeType: audio.mimeType,
          sharePositionOrigin: origin,
        );
      },
    ),
  ];
}
