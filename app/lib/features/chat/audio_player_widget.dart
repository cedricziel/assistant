import 'dart:typed_data';

import 'package:audioplayers/audioplayers.dart';
import 'package:flutter/material.dart';

/// A compact play/stop button that fetches and plays audio on demand.
///
/// Audio is fetched lazily via [fetchAudio] on first play.  Subsequent taps
/// replay the cached bytes without an extra network round-trip.
class AudioPlayerWidget extends StatefulWidget {
  const AudioPlayerWidget({super.key, required this.fetchAudio});

  /// Called once to obtain the audio bytes and MIME type.  Returns `null` on failure.
  final Future<({Uint8List bytes, String mimeType})?> Function() fetchAudio;

  @override
  State<AudioPlayerWidget> createState() => _AudioPlayerWidgetState();
}

class _AudioPlayerWidgetState extends State<AudioPlayerWidget> {
  final _player = AudioPlayer();
  bool _isPlaying = false;
  bool _isLoading = false;
  bool _hasError = false;
  ({Uint8List bytes, String mimeType})? _cachedAudio;

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

    setState(() {
      _isLoading = true;
      _hasError = false;
    });
    try {
      _cachedAudio ??= await widget.fetchAudio();
      final audio = _cachedAudio;
      if (audio == null || !mounted) {
        if (mounted) {
          setState(() {
            _isLoading = false;
            _hasError = true;
          });
        }
        return;
      }

      await _player.play(BytesSource(audio.bytes, mimeType: audio.mimeType));
      if (mounted) {
        setState(() {
          _isPlaying = true;
          _isLoading = false;
        });
      }
    } catch (_) {
      _cachedAudio = null; // clear so retry re-fetches
      if (mounted) {
        setState(() {
          _isLoading = false;
          _hasError = true;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_isLoading) {
      return const SizedBox(
        width: 20,
        height: 20,
        child: CircularProgressIndicator.adaptive(strokeWidth: 2),
      );
    }
    if (_hasError) {
      return IconButton(
        key: const Key('audio_error_retry'),
        icon: Icon(
          Icons.error_outline,
          color: Theme.of(context).colorScheme.error,
        ),
        onPressed: _toggle,
        iconSize: 20,
        padding: EdgeInsets.zero,
        constraints: const BoxConstraints(),
        tooltip: 'Failed to load audio — tap to retry',
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
