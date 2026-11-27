import 'package:material_ui/material_ui.dart';

/// Animated three-dot indicator shown while waiting for the first token.
///
/// With `MediaQuery.disableAnimations` the dots stay still and the ticker is
/// stopped, so the indicator schedules no frames.
class TypingIndicator extends StatelessWidget {
  const TypingIndicator({super.key});

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: 'typing',
      child: const SizedBox(
        height: 16,
        width: 40,
        child: Row(
          mainAxisAlignment: MainAxisAlignment.spaceEvenly,
          children: [
            _Dot(),
            _Dot(delay: Duration(milliseconds: 150)),
            _Dot(delay: Duration(milliseconds: 300)),
          ],
        ),
      ),
    );
  }
}

class _Dot extends StatefulWidget {
  const _Dot({this.delay = Duration.zero});
  final Duration delay;

  @override
  State<_Dot> createState() => _DotState();
}

class _DotState extends State<_Dot> with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _opacity;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      duration: const Duration(milliseconds: 1200),
      vsync: this,
    );

    // Stagger offset as a fraction of total duration.
    final delay = widget.delay.inMilliseconds / 1200;
    final end = (delay + 0.5).clamp(0.0, 1.0);

    _opacity =
        TweenSequence<double>([
          TweenSequenceItem(
            tween: Tween(
              begin: 0.2,
              end: 1.0,
            ).chain(CurveTween(curve: Curves.easeInOut)),
            weight: 50,
          ),
          TweenSequenceItem(
            tween: Tween(
              begin: 1.0,
              end: 0.2,
            ).chain(CurveTween(curve: Curves.easeInOut)),
            weight: 50,
          ),
        ]).animate(
          CurvedAnimation(parent: _controller, curve: Interval(delay, end)),
        );
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    // A repeating ticker schedules frames forever, so stop it entirely under
    // reduced motion instead of merely skipping the animated build.
    if (MediaQuery.disableAnimationsOf(context)) {
      _controller.stop();
    } else if (!_controller.isAnimating) {
      _controller.repeat();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final color = Theme.of(context).colorScheme.onSurfaceVariant;

    if (MediaQuery.of(context).disableAnimations) {
      return Container(
        width: 6,
        height: 6,
        decoration: BoxDecoration(color: color, shape: BoxShape.circle),
      );
    }

    return AnimatedBuilder(
      animation: _opacity,
      builder: (context, child) =>
          Opacity(opacity: _opacity.value, child: child),
      child: Container(
        width: 6,
        height: 6,
        decoration: BoxDecoration(color: color, shape: BoxShape.circle),
      ),
    );
  }
}
