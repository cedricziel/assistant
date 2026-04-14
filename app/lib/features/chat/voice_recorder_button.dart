import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:record/record.dart';

/// A mic button that records audio from the microphone.
///
/// While recording it shows a timer and a stop button.  When stopped it
/// invokes [onRecordingComplete] with the raw opus bytes and MIME type.
class VoiceRecorderButton extends StatefulWidget {
  const VoiceRecorderButton({
    super.key,
    required this.onRecordingComplete,
    required this.onError,
  });

  final void Function(Uint8List bytes, String mimeType) onRecordingComplete;
  final void Function(String error) onError;

  @override
  State<VoiceRecorderButton> createState() => _VoiceRecorderButtonState();
}

class _VoiceRecorderButtonState extends State<VoiceRecorderButton> {
  final _recorder = AudioRecorder();
  bool _isRecording = false;
  int _secondsElapsed = 0;
  Timer? _countdownTimer;
  static const _maxSeconds = 120;

  StreamSubscription<List<int>>? _streamSub;
  final List<int> _recordedBytes = [];

  @override
  void dispose() {
    _countdownTimer?.cancel();
    _streamSub?.cancel();
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

  Future<void> _start() async {
    final hasPermission = await _recorder.hasPermission();
    if (!hasPermission) {
      widget.onError('Microphone access is required to send voice messages');
      return;
    }
    try {
      _recordedBytes.clear();
      final stream = await _recorder.startStream(
        const RecordConfig(encoder: AudioEncoder.opus, numChannels: 1),
      );
      _streamSub = stream.listen((chunk) => _recordedBytes.addAll(chunk));
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
    await _streamSub?.cancel();
    _streamSub = null;
    await _recorder.stop();
    final bytes = Uint8List.fromList(_recordedBytes);
    setState(() => _isRecording = false);
    if (bytes.isNotEmpty) {
      widget.onRecordingComplete(bytes, 'audio/webm');
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
