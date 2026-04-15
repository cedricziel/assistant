import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:record/record.dart';

import 'recorder_bytes_io.dart'
    if (dart.library.js_interop) 'recorder_bytes_web.dart';

/// A mic button that records audio from the microphone.
///
/// While recording it shows a countdown timer and a stop button.  When stopped
/// it invokes [onRecordingComplete] with the raw audio bytes and MIME type.
///
/// Encoder / container selection:
/// - **Web Chrome/Firefox**: Opus (MediaRecorder picks WebM or OGG container).
/// - **Web Safari**: AAC-LC in MP4 container (Opus not supported).
/// - **Web fallback**: PCM/WAV.
/// - **Native (iOS, macOS, Android)**: AAC-LC → audio/aac.
/// - **Native fallback**: Opus → audio/ogg, then WAV → audio/wav.
class VoiceRecorderButton extends StatefulWidget {
  const VoiceRecorderButton({
    super.key,
    required this.onRecordingComplete,
    required this.onError,
    @visibleForTesting this.audioRecorder,
  });

  final void Function(Uint8List bytes, String mimeType) onRecordingComplete;
  final void Function(String error) onError;

  /// Overrides the default [AudioRecorder] instance. Intended for testing only.
  @visibleForTesting
  final AudioRecorder? audioRecorder;

  @override
  State<VoiceRecorderButton> createState() => _VoiceRecorderButtonState();
}

class _VoiceRecorderButtonState extends State<VoiceRecorderButton> {
  late final AudioRecorder _recorder;
  bool _isRecording = false;
  int _secondsElapsed = 0;
  Timer? _countdownTimer;
  static const _maxSeconds = 120;

  /// MIME type determined at record-start time, used when reading bytes.
  String _selectedMime = 'audio/webm';

  @override
  void initState() {
    super.initState();
    _recorder = widget.audioRecorder ?? AudioRecorder();
  }

  @override
  void dispose() {
    _countdownTimer?.cancel();
    _recorder.dispose();
    super.dispose();
  }

  Future<void> _toggle() async {
    if (_isRecording) {
      await _stop();
    } else {
      await _start();
    }
  }

  /// Returns the best available encoder and its MIME type for the current
  /// platform/browser.
  Future<(AudioEncoder, String)> _selectEncoder() async {
    if (kIsWeb) {
      // Prefer Opus — supported by Chrome (WebM) and Firefox (OGG).
      if (await _recorder.isEncoderSupported(AudioEncoder.opus)) {
        // Actual container is determined by the browser's MediaRecorder;
        // we use a conservative fallback MIME here — the web helper will
        // read the real content-type from the blob response instead.
        return (AudioEncoder.opus, 'audio/webm');
      }
      // Safari: AAC inside MP4.
      if (await _recorder.isEncoderSupported(AudioEncoder.aacLc)) {
        return (AudioEncoder.aacLc, 'audio/mp4');
      }
      return (AudioEncoder.wav, 'audio/wav');
    } else {
      // Native platforms: AAC is the most widely supported efficient codec.
      if (await _recorder.isEncoderSupported(AudioEncoder.aacLc)) {
        return (AudioEncoder.aacLc, 'audio/aac');
      }
      if (await _recorder.isEncoderSupported(AudioEncoder.opus)) {
        return (AudioEncoder.opus, 'audio/ogg');
      }
      return (AudioEncoder.wav, 'audio/wav');
    }
  }

  /// Returns a suitable file path for the recording on native platforms.
  /// On web this value is ignored by the record package.
  Future<String> _tempPath(AudioEncoder encoder) async {
    if (kIsWeb) return '';
    final ext = switch (encoder) {
      AudioEncoder.aacLc => 'm4a',
      AudioEncoder.opus => 'ogg',
      AudioEncoder.wav => 'wav',
      _ => 'audio',
    };
    final dir = await getTemporaryDirectory();
    return '${dir.path}/assistant_voice_${DateTime.now().millisecondsSinceEpoch}.$ext';
  }

  Future<void> _start() async {
    try {
      final hasPermission = await _recorder.hasPermission();
      if (!hasPermission) {
        widget.onError('Microphone access is required to send voice messages');
        return;
      }
    } catch (_) {
      widget.onError('Microphone access is required to send voice messages');
      return;
    }

    try {
      final (encoder, mime) = await _selectEncoder();
      final path = await _tempPath(encoder);
      _selectedMime = mime;

      await _recorder.start(
        RecordConfig(encoder: encoder, numChannels: 1),
        path: path,
      );

      setState(() {
        _isRecording = true;
        _secondsElapsed = 0;
      });

      _countdownTimer = Timer.periodic(const Duration(seconds: 1), (_) {
        setState(() => _secondsElapsed++);
        if (_secondsElapsed >= _maxSeconds) _stop();
      });
    } catch (e) {
      widget.onError('Failed to start recording: $e');
    }
  }

  Future<void> _stop() async {
    _countdownTimer?.cancel();
    final urlOrPath = await _recorder.stop();
    setState(() => _isRecording = false);

    if (urlOrPath == null || urlOrPath.isEmpty) return;

    try {
      final (bytes, mime) = await readRecordedOutput(urlOrPath, _selectedMime);
      if (bytes.isNotEmpty) {
        widget.onRecordingComplete(bytes, mime);
      }
    } catch (e) {
      widget.onError('Failed to read recording: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_isRecording) {
      final remaining = _maxSeconds - _secondsElapsed;
      final minutes = remaining ~/ 60;
      final secs = remaining % 60;
      return Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 8,
            height: 8,
            decoration: const BoxDecoration(
              color: Colors.red,
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 6),
          Text(
            '$minutes:${secs.toString().padLeft(2, '0')}',
            style: const TextStyle(fontSize: 13, color: Colors.red),
          ),
          const SizedBox(width: 4),
          IconButton(
            icon: const Icon(Icons.stop_circle_outlined, color: Colors.red),
            onPressed: _stop,
            tooltip: 'Stop recording',
          ),
        ],
      );
    }
    return IconButton(
      icon: const Icon(Icons.mic_none_outlined),
      tooltip: 'Send voice message',
      onPressed: _toggle,
    );
  }
}
