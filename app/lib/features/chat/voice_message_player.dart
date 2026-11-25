import 'dart:async';

import 'package:audioplayers/audioplayers.dart';
import 'package:flutter/services.dart';

import '../../shared/platform/widgets.dart';

class VoiceMessagePlayer extends StatefulWidget {
  const VoiceMessagePlayer({
    super.key,
    required this.audioBytes,
    required this.transcript,
    required this.foregroundColor,
    this.audioMimeType,
  });

  final Uint8List audioBytes;
  final String transcript;
  final Color foregroundColor;
  final String? audioMimeType;

  @override
  State<VoiceMessagePlayer> createState() => _VoiceMessagePlayerState();
}

class _VoiceMessagePlayerState extends State<VoiceMessagePlayer> {
  final _player = AudioPlayer();
  bool _isPlaying = false;
  Duration _position = Duration.zero;
  Duration _duration = Duration.zero;
  bool _transcriptExpanded = false;

  @override
  void initState() {
    super.initState();
    _player.onPlayerComplete.listen((_) {
      if (mounted) {
        setState(() {
          _isPlaying = false;
          _position = Duration.zero;
        });
      }
    });
    _player.onPositionChanged.listen((p) {
      if (mounted) setState(() => _position = p);
    });
    _player.onDurationChanged.listen((d) {
      if (mounted) setState(() => _duration = d);
    });
  }

  @override
  void dispose() {
    _player.dispose();
    super.dispose();
  }

  Future<void> _toggle() async {
    if (_isPlaying) {
      await _player.pause();
      setState(() => _isPlaying = false);
    } else {
      if (_position == Duration.zero) {
        await _player.play(
          BytesSource(widget.audioBytes, mimeType: widget.audioMimeType),
        );
      } else {
        await _player.resume();
      }
      setState(() => _isPlaying = true);
    }
  }

  String _formatDuration(Duration d) {
    final minutes = d.inMinutes;
    final seconds = d.inSeconds % 60;
    return '$minutes:${seconds.toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    final fg = widget.foregroundColor;
    final hasTranscript =
        widget.transcript.trim().isNotEmpty &&
        widget.transcript.trim() != '🎤 Voice message';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        // Player row: play/pause + progress + duration.
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            GestureDetector(
              onTap: _toggle,
              child: Icon(
                _isPlaying
                    ? Icons.pause_circle_filled
                    : Icons.play_circle_filled,
                color: fg,
                size: 28,
              ),
            ),
            const SizedBox(width: 8),
            SizedBox(
              width: 120,
              child: SliderTheme(
                data: SliderThemeData(
                  trackHeight: 3,
                  thumbShape: const RoundSliderThumbShape(
                    enabledThumbRadius: 5,
                  ),
                  activeTrackColor: fg,
                  inactiveTrackColor: fg.withValues(alpha: 0.3),
                  thumbColor: fg,
                  overlayShape: SliderComponentShape.noOverlay,
                ),
                child: Slider(
                  value: _duration.inMilliseconds > 0
                      ? _position.inMilliseconds / _duration.inMilliseconds
                      : 0,
                  onChanged: (v) {
                    final target = Duration(
                      milliseconds: (v * _duration.inMilliseconds).round(),
                    );
                    _player.seek(target);
                  },
                ),
              ),
            ),
            const SizedBox(width: 6),
            Text(
              _formatDuration(
                _isPlaying || _position > Duration.zero ? _position : _duration,
              ),
              style: TextStyle(fontSize: 11, color: fg),
            ),
          ],
        ),
        // Collapsible transcript.
        if (hasTranscript) ...[
          const SizedBox(height: 4),
          GestureDetector(
            onTap: () =>
                setState(() => _transcriptExpanded = !_transcriptExpanded),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  _transcriptExpanded ? Icons.expand_less : Icons.expand_more,
                  size: 16,
                  color: fg.withValues(alpha: 0.7),
                ),
                const SizedBox(width: 4),
                Flexible(
                  child: Text(
                    _transcriptExpanded
                        ? widget.transcript
                        : _truncate(widget.transcript, 40),
                    style: TextStyle(
                      fontSize: 12,
                      color: fg.withValues(alpha: 0.7),
                      fontStyle: FontStyle.italic,
                    ),
                    maxLines: _transcriptExpanded ? null : 1,
                    overflow: _transcriptExpanded
                        ? TextOverflow.visible
                        : TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
          ),
        ],
      ],
    );
  }

  static String _truncate(String text, int maxLen) {
    if (text.length <= maxLen) return text;
    return '${text.substring(0, maxLen)}\u2026';
  }
}
