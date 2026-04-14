import 'dart:typed_data';

import 'package:audioplayers/audioplayers.dart';
import 'package:flutter/material.dart';

/// A compact play/stop button that fetches and plays audio on demand.
///
/// Audio is fetched lazily via [fetchAudio] on first play.  Subsequent taps
/// replay the cached bytes without an extra network round-trip.
class AudioPlayerWidget extends StatefulWidget {
  const AudioPlayerWidget({super.key, required this.fetchAudio});

  /// Called once to obtain the audio bytes.  Returns `null` on failure.
  final Future<Uint8List?> Function() fetchAudio;

  @override
  State<AudioPlayerWidget> createState() => _AudioPlayerWidgetState();
}

class _AudioPlayerWidgetState extends State<AudioPlayerWidget> {
  final _player = AudioPlayer();
  bool _isPlaying = false;
  bool _isLoading = false;
  Uint8List? _cachedBytes;

  @override
  void initState() {
    super.initState();
    _player.onPlayerComplete.listen((_) {
      if (mounted) setState(() => _isPlaying = false);
    });
  }

  @override
  void dispose() {
    _player.dispose();
    super.dispose();
  }

  Future<void> _toggle() async {
    if (_isPlaying) {
      await _player.stop();
      setState(() => _isPlaying = false);
      return;
    }

    setState(() => _isLoading = true);
    try {
      _cachedBytes ??= await widget.fetchAudio();
      final bytes = _cachedBytes;
      if (bytes == null || !mounted) {
        setState(() => _isLoading = false);
        return;
      }

      await _player.play(BytesSource(bytes));
      if (mounted) {
        setState(() {
          _isPlaying = true;
          _isLoading = false;
        });
      }
    } catch (_) {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_isLoading) {
      return const SizedBox(
        width: 20,
        height: 20,
        child: CircularProgressIndicator(strokeWidth: 2),
      );
    }
    return IconButton(
      icon: Icon(
        _isPlaying ? Icons.stop_circle_outlined : Icons.play_circle_outlined,
      ),
      onPressed: _toggle,
      iconSize: 20,
      padding: EdgeInsets.zero,
      constraints: const BoxConstraints(),
      tooltip: _isPlaying ? 'Stop' : 'Play audio response',
    );
  }
}
