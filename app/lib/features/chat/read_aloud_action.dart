import 'dart:async';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:audioplayers/audioplayers.dart';
import 'package:flutter/services.dart';

import '../../shared/platform/widgets.dart';

/// State for the read-aloud TTS action.
enum _ReadAloudState { idle, loading, playing, error }

/// Stateful meta action that fetches on-demand TTS and plays it back.
class ReadAloudAction extends StatefulWidget {
  const ReadAloudAction({super.key, required this.fetchAudio});

  final Future<({Uint8List bytes, String mimeType})?> Function() fetchAudio;

  @override
  State<ReadAloudAction> createState() => _ReadAloudActionState();
}

class _ReadAloudActionState extends State<ReadAloudAction> {
  final _player = AudioPlayer();
  _ReadAloudState _state = _ReadAloudState.idle;
  ({Uint8List bytes, String mimeType})? _cachedAudio;
  Timer? _errorTimer;

  @override
  void initState() {
    super.initState();
    _player.onPlayerComplete.listen((_) {
      if (mounted) setState(() => _state = _ReadAloudState.idle);
    });
  }

  @override
  void dispose() {
    _errorTimer?.cancel();
    _player.dispose();
    super.dispose();
  }

  Future<void> _toggle() async {
    switch (_state) {
      case _ReadAloudState.playing:
        await _player.stop();
        setState(() => _state = _ReadAloudState.idle);
      case _ReadAloudState.idle:
      case _ReadAloudState.error:
        setState(() => _state = _ReadAloudState.loading);
        try {
          _cachedAudio ??= await widget.fetchAudio();
          final audio = _cachedAudio;
          if (audio == null || !mounted) {
            _showError();
            return;
          }
          await _player.play(
            BytesSource(audio.bytes, mimeType: audio.mimeType),
          );
          if (mounted) setState(() => _state = _ReadAloudState.playing);
        } catch (_) {
          _showError();
        }
      case _ReadAloudState.loading:
        break; // ignore taps while loading
    }
  }

  void _showError() {
    if (!mounted) return;
    setState(() => _state = _ReadAloudState.error);
    _errorTimer?.cancel();
    _errorTimer = Timer(const Duration(seconds: 4), () {
      if (mounted) setState(() => _state = _ReadAloudState.idle);
    });
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final errorColor = scheme.error;
    final mutedColor = scheme.onSurfaceVariant;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        switch (_state) {
          _ReadAloudState.idle => MessageActionButton(
            icon: Icons.volume_up_outlined,
            label: 'Read aloud',
            onPressed: _toggle,
          ),
          _ReadAloudState.loading => Padding(
            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                SizedBox(
                  width: 14,
                  height: 14,
                  child: CircularProgressIndicator.adaptive(
                    strokeWidth: 2,
                    valueColor: AlwaysStoppedAnimation<Color>(mutedColor),
                  ),
                ),
                const SizedBox(width: 4),
                Text(
                  'Loading\u2026',
                  style: TextStyle(fontSize: 12, color: mutedColor),
                ),
              ],
            ),
          ),
          _ReadAloudState.playing => MessageActionButton(
            icon: Icons.stop_rounded,
            label: 'Stop reading',
            onPressed: _toggle,
          ),
          _ReadAloudState.error => MessageActionButton(
            icon: Icons.volume_up_outlined,
            label: 'Read aloud',
            onPressed: _toggle,
          ),
        },
        if (_state == _ReadAloudState.error)
          Padding(
            padding: const EdgeInsets.only(top: 2, left: 4),
            child: Text(
              'Could not generate audio',
              style: TextStyle(fontSize: 11, color: errorColor),
            ),
          ),
      ],
    );
  }
}
