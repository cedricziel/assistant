import 'package:material_ui/material_ui.dart';

/// Vertical frame of a chat screen, top to bottom: [banners], the [thread]
/// (which takes all remaining space), [aboveComposer], then the [composer].
///
/// [banners] hold the error banner and history-loading indicator;
/// [aboveComposer] holds the reconnect banner first, then the turn progress
/// card, then anything siblings append (follow-up chips). Empty lists render
/// nothing.
class ChatThreadLayout extends StatelessWidget {
  const ChatThreadLayout({
    super.key,
    this.banners = const [],
    required this.thread,
    this.aboveComposer = const [],
    required this.composer,
  });

  final List<Widget> banners;
  final Widget thread;
  final List<Widget> aboveComposer;
  final Widget composer;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        ...banners,
        Expanded(child: thread),
        ...aboveComposer,
        composer,
      ],
    );
  }
}
