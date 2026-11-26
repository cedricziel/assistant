import 'package:flutter/scheduler.dart';
import 'package:material_ui/material_ui.dart';

/// Distance from the end of the thread within which the viewport counts as
/// "at the bottom" and keeps following new content.
const double _kBottomThreshold = 80.0;

/// Distance within which [ThreadViewportController.scrollToBottomIfNear]
/// scrolls.
const double _kNearThreshold = 200.0;

const Duration _kScrollDuration = Duration(milliseconds: 200);

/// Imperative handle for a [ThreadViewport].
///
/// Requests made while no viewport is attached (for instance while the thread
/// shows its empty state) are ignored.
class ThreadViewportController {
  _ThreadViewportState? _state;

  /// Scrolls to the end after the next frame, even when the user has scrolled
  /// away. Use it when a different conversation opens or a message is sent.
  void scrollToBottom() => _state?._scrollToBottom();

  /// Scrolls to the end only when it is within 200 dp, so a user reading
  /// history keeps their position. Use it when the keyboard opens.
  void scrollToBottomIfNear() => _state?._scrollToBottomIfNear();
}

/// Scrolling message list that follows new content while the user is at the
/// bottom and offers a "scroll to bottom" button once they scroll away.
///
/// [revision] is the signal that content changed. Anything that grows the
/// thread without changing [itemCount] (streaming tokens, entries mutated in
/// place) must bump it; a higher value scrolls to the end if the viewport is
/// within 80 dp of it, and is ignored otherwise.
class ThreadViewport extends StatefulWidget {
  const ThreadViewport({
    super.key,
    required this.itemCount,
    required this.itemBuilder,
    required this.revision,
    this.controller,
    this.padding = const EdgeInsets.symmetric(vertical: 16, horizontal: 12),
  });

  final int itemCount;
  final IndexedWidgetBuilder itemBuilder;
  final int revision;
  final ThreadViewportController? controller;
  final EdgeInsetsGeometry padding;

  @override
  State<ThreadViewport> createState() => _ThreadViewportState();
}

class _ThreadViewportState extends State<ThreadViewport> {
  final _scrollController = ScrollController();
  bool _atBottom = true;

  @override
  void initState() {
    super.initState();
    _scrollController.addListener(_onScroll);
    widget.controller?._state = this;
    // A thread that appears with content (a conversation just loaded) opens
    // at its end.
    if (widget.itemCount > 0) _scrollToBottom();
  }

  @override
  void didUpdateWidget(ThreadViewport oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      if (oldWidget.controller?._state == this) {
        oldWidget.controller?._state = null;
      }
      widget.controller?._state = this;
    }
    if (widget.revision != oldWidget.revision &&
        _atBottom &&
        widget.itemCount > 0) {
      _scrollToBottom();
    }
  }

  @override
  void dispose() {
    if (widget.controller?._state == this) widget.controller?._state = null;
    _scrollController
      ..removeListener(_onScroll)
      ..dispose();
    super.dispose();
  }

  void _onScroll() {
    if (!_scrollController.hasClients) return;
    final pos = _scrollController.position;
    final nearBottom = pos.pixels >= pos.maxScrollExtent - _kBottomThreshold;
    if (_atBottom != nearBottom) setState(() => _atBottom = nearBottom);
  }

  void _scrollToBottom() {
    SchedulerBinding.instance.ensureVisualUpdate();
    SchedulerBinding.instance.addPostFrameCallback((_) {
      if (!mounted || !_scrollController.hasClients) return;
      _animateToEnd();
    });
  }

  void _scrollToBottomIfNear() {
    if (!_scrollController.hasClients) return;
    final pos = _scrollController.position;
    if (pos.maxScrollExtent - pos.pixels < _kNearThreshold) _animateToEnd();
  }

  void _animateToEnd() {
    _scrollController.animateTo(
      _scrollController.position.maxScrollExtent,
      duration: _kScrollDuration,
      curve: Curves.easeOut,
    );
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        ListView.builder(
          controller: _scrollController,
          padding: widget.padding,
          itemCount: widget.itemCount,
          itemBuilder: widget.itemBuilder,
        ),
        if (!_atBottom)
          Positioned(
            bottom: 8,
            right: 8,
            child: FloatingActionButton.small(
              key: const Key('scroll_to_bottom_button'),
              onPressed: _scrollToBottom,
              tooltip: 'Scroll to bottom',
              child: const Icon(Icons.keyboard_arrow_down),
            ),
          ),
      ],
    );
  }
}
