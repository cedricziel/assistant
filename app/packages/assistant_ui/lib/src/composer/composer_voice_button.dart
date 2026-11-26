import 'package:material_ui/material_ui.dart';

/// The mic affordance of the composer, in its two states.
///
/// Idle it shows a mic button that fires [onStart]. While [isRecording] it
/// shows a recording dot, the time left (counting down from [maxDuration] as
/// [elapsed] grows) and a stop button that fires [onStop].
///
/// Presentation only: the app owns the recorder, the timer and the
/// permission flow, and drives this button through its parameters.
class ComposerVoiceButton extends StatelessWidget {
  const ComposerVoiceButton({
    super.key,
    required this.isRecording,
    required this.onStart,
    required this.onStop,
    this.elapsed = Duration.zero,
    this.maxDuration = const Duration(seconds: 120),
  });

  final bool isRecording;

  /// Time recorded so far; only read while [isRecording].
  final Duration elapsed;

  /// The recording limit the countdown starts from.
  final Duration maxDuration;

  final VoidCallback onStart;
  final VoidCallback onStop;

  @override
  Widget build(BuildContext context) {
    if (!isRecording) {
      return IconButton(
        icon: const Icon(Icons.mic_none_outlined),
        tooltip: 'Send voice message',
        onPressed: onStart,
      );
    }
    final error = Theme.of(context).colorScheme.error;
    final remaining = (maxDuration - elapsed).inSeconds;
    final minutes = remaining ~/ 60;
    final secs = remaining % 60;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 8,
          height: 8,
          decoration: BoxDecoration(color: error, shape: BoxShape.circle),
        ),
        const SizedBox(width: 6),
        Text(
          '$minutes:${secs.toString().padLeft(2, '0')}',
          style: TextStyle(fontSize: 13, color: error),
        ),
        const SizedBox(width: 4),
        IconButton(
          icon: Icon(Icons.stop_circle_outlined, color: error),
          onPressed: onStop,
          tooltip: 'Stop recording',
        ),
      ],
    );
  }
}
