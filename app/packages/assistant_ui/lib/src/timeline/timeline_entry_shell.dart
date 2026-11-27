import 'dart:async';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../models/timeline_state.dart';
import '../scopes/thread_density_scope.dart';
import '../theme/assistant_ui_colors.dart';

/// The chrome of one timeline entry (thinking, tool call, subagent): a header
/// pill, a collapsible [body] and an optional [footer].
///
/// The shell is the single owner of presentation state: expansion, user
/// pinning, auto-collapse timing, reduced motion, stale opacity and the
/// reaction to [EntryState]. Bodies are plain widgets that render their own
/// fields and hold no such state.
///
/// The [footer] renders below the collapsible region, outside it, so it stays
/// visible while the entry is collapsed. It is laid out at the full entry
/// width (at most 640 logical pixels) and hosts any widget, for example an
/// approval card or an app-side frame.
class TimelineEntryShell extends StatefulWidget {
  const TimelineEntryShell({
    super.key,
    required this.icon,
    required this.label,
    this.status,
    this.entryState = EntryState.complete,
    this.failed = false,
    this.expandable = false,
    this.forceExpand = false,
    this.body,
    this.footer,
  });

  /// Key of the header pill, for tests that inspect its decoration.
  static const headerKey = ValueKey<String>('timeline-entry-shell-header');

  /// Delay between an entry finishing and its auto-collapse.
  static const autoCollapseDelay = Duration(milliseconds: 500);

  final IconData icon;
  final String label;

  /// Status indicator shown after the label. At compact density a bare
  /// [SizedBox] (a spinner placeholder) is hidden.
  final Widget? status;

  final EntryState entryState;

  /// Whether the entry's work ended in an error. The header's semantics label
  /// appends ", failed" then, since [EntryState] has no error value.
  final bool failed;

  /// Whether tapping the header toggles the [body].
  final bool expandable;

  /// Shows the [body] regardless of the expansion state.
  final bool forceExpand;

  /// The collapsible content.
  final Widget? body;

  /// Content below the collapsible region; visible while collapsed.
  final Widget? footer;

  /// Wraps [child] in a height-capped, bottom-faded scroll view. With
  /// [isActive] the view follows the newest content at the bottom.
  static Widget fadeContainer({
    required double maxHeight,
    required bool isActive,
    required Widget child,
  }) {
    return ConstrainedBox(
      constraints: BoxConstraints(maxHeight: maxHeight),
      child: ShaderMask(
        shaderCallback: (bounds) => const LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [
            AssistantUiColors.fadeMask,
            AssistantUiColors.fadeMask,
            Colors.transparent,
          ],
          stops: [0.0, 0.8, 1.0],
        ).createShader(bounds),
        blendMode: BlendMode.dstIn,
        child: SingleChildScrollView(reverse: isActive, child: child),
      ),
    );
  }

  @override
  State<TimelineEntryShell> createState() => _TimelineEntryShellState();
}

class _TimelineEntryShellState extends State<TimelineEntryShell> {
  bool _expanded = false;
  bool _userPinned = false;
  bool _hasFocus = false;
  bool _showFocusHighlight = false;
  Timer? _collapseTimer;

  @override
  void initState() {
    super.initState();
    // `initState` cannot subscribe to the scope, so it reads the density once.
    final density =
        context.getInheritedWidgetOfExactType<ThreadDensityScope>()?.density ??
        TimelineDensity.normal;
    _expanded =
        widget.entryState == EntryState.awaitingInput ||
        (widget.entryState == EntryState.active &&
            density != TimelineDensity.compact);
  }

  @override
  void didUpdateWidget(covariant TimelineEntryShell oldWidget) {
    super.didUpdateWidget(oldWidget);
    final state = widget.entryState;
    final previous = oldWidget.entryState;
    if (state == previous) return;

    switch (state) {
      case EntryState.awaitingInput:
        _collapseTimer?.cancel();
        setState(() => _expanded = true);
      case EntryState.active:
        _collapseTimer?.cancel();
        if (!_userPinned &&
            previous != EntryState.awaitingInput &&
            ThreadDensityScope.of(context) != TimelineDensity.compact) {
          setState(() => _expanded = true);
        }
      case EntryState.complete:
        if (previous == EntryState.active ||
            previous == EntryState.awaitingInput) {
          _scheduleAutoCollapse();
        }
      case EntryState.stale:
        _collapseTimer?.cancel();
        if (!_userPinned) setState(() => _expanded = false);
    }
  }

  @override
  void dispose() {
    _collapseTimer?.cancel();
    super.dispose();
  }

  void _scheduleAutoCollapse() {
    _collapseTimer?.cancel();
    if (_userPinned) return;

    if (MediaQuery.of(context).disableAnimations) {
      setState(() => _expanded = false);
      return;
    }

    _collapseTimer = Timer(TimelineEntryShell.autoCollapseDelay, () {
      if (mounted && !_userPinned) {
        setState(() => _expanded = false);
      }
    });
  }

  void _onTap() {
    // An entry awaiting input must stay open until it is answered.
    if (widget.entryState == EntryState.awaitingInput) return;
    setState(() {
      _expanded = !_expanded;
      _userPinned = _expanded;
    });
  }

  /// The body region. Under reduced motion it skips [AnimatedSize]: with a zero
  /// duration it re-dirties itself during layout when the body toggles.
  Widget _collapsible(
    ColorScheme colorScheme, {
    required bool animate,
    required bool isExpanded,
  }) {
    final body = widget.body;
    final content = isExpanded && body != null
        ? Padding(
            padding: const EdgeInsets.only(left: 12, top: 4, bottom: 4),
            child: Container(
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerLowest,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(
                  color: colorScheme.outlineVariant.withAlpha(80),
                  width: 0.5,
                ),
              ),
              child: body,
            ),
          )
        : const SizedBox.shrink();
    if (!animate) return content;
    return AnimatedSize(
      duration: const Duration(milliseconds: 200),
      curve: Curves.easeInOut,
      child: content,
    );
  }

  /// The header's spoken status: pending input first, then an error, then the
  /// entry state.
  String get _statusLabel {
    if (widget.entryState == EntryState.awaitingInput) return 'awaiting input';
    if (widget.failed) return 'failed';
    return switch (widget.entryState) {
      EntryState.active => 'running',
      EntryState.complete => 'done',
      EntryState.stale => 'stale',
      EntryState.awaitingInput => 'awaiting input',
    };
  }

  Widget _header(
    ColorScheme colorScheme, {
    required bool isExpanded,
    required bool showStatus,
    required Widget? status,
    required bool awaitingInput,
  }) {
    final borderRadius = BorderRadius.circular(12);
    final pill = Container(
      key: TimelineEntryShell.headerKey,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withAlpha(120),
        borderRadius: borderRadius,
      ),
      // Painted over the pill so the accent and the focus ring move no layout.
      foregroundDecoration: awaitingInput
          ? BoxDecoration(
              borderRadius: borderRadius,
              border: Border.all(color: colorScheme.primary, width: 1.5),
            )
          : _showFocusHighlight
          ? BoxDecoration(
              borderRadius: borderRadius,
              border: Border.all(color: colorScheme.primary, width: 2),
            )
          : null,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(widget.icon, size: 14, color: colorScheme.onSurfaceVariant),
          const SizedBox(width: 6),
          Flexible(
            child: Text(
              widget.label,
              style: TextStyle(
                fontSize: 12,
                color: colorScheme.onSurfaceVariant,
                fontWeight: FontWeight.w500,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          if (showStatus && status != null) ...[
            const SizedBox(width: 6),
            status,
          ],
          if (widget.expandable) ...[
            const SizedBox(width: 4),
            Icon(
              isExpanded
                  ? Icons.expand_less_rounded
                  : Icons.expand_more_rounded,
              size: 14,
              color: colorScheme.onSurfaceVariant.withAlpha(150),
            ),
          ],
        ],
      ),
    );

    final control = widget.expandable
        ? FocusableActionDetector(
            mouseCursor: SystemMouseCursors.click,
            shortcuts: const <ShortcutActivator, Intent>{
              SingleActivator(LogicalKeyboardKey.enter): ActivateIntent(),
              SingleActivator(LogicalKeyboardKey.space): ActivateIntent(),
            },
            actions: <Type, Action<Intent>>{
              ActivateIntent: CallbackAction<ActivateIntent>(
                onInvoke: (_) {
                  _onTap();
                  return null;
                },
              ),
            },
            onFocusChange: (focused) {
              if (focused == _hasFocus) return;
              setState(() => _hasFocus = focused);
            },
            onShowFocusHighlight: (show) {
              if (show == _showFocusHighlight) return;
              setState(() => _showFocusHighlight = show);
            },
            child: InkWell(
              // The enclosing detector owns the single focus stop.
              canRequestFocus: false,
              // Expansion is its own feedback; a lingering ripple would sit
              // behind the translucent pill.
              splashFactory: NoSplash.splashFactory,
              onTap: _onTap,
              borderRadius: borderRadius,
              child: pill,
            ),
          )
        : pill;

    // One node for the whole header: the pill's label, status and icons are
    // covered by it, and the detector's focus is mirrored here so the node is a
    // single focus stop.
    return Semantics(
      container: true,
      excludeSemantics: true,
      label: '${widget.label}, $_statusLabel',
      button: widget.expandable,
      expanded: isExpanded,
      focusable: widget.expandable,
      focused: widget.expandable ? _hasFocus : null,
      onTap: widget.expandable ? _onTap : null,
      child: control,
    );
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final density = ThreadDensityScope.of(context);
    final isExpanded = widget.forceExpand || _expanded;
    final reducedMotion = MediaQuery.of(context).disableAnimations;
    final awaitingInput = widget.entryState == EntryState.awaitingInput;
    final status = widget.status;

    // In compact density, only show icon + name.
    final showStatus =
        density != TimelineDensity.compact || status is! SizedBox;
    final footer = widget.footer;

    return Opacity(
      opacity: widget.entryState == EntryState.stale ? 0.6 : 1.0,
      child: Align(
        alignment: Alignment.centerLeft,
        child: Container(
          margin: const EdgeInsets.symmetric(vertical: 2),
          constraints: const BoxConstraints(maxWidth: 640),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              _header(
                colorScheme,
                isExpanded: isExpanded,
                showStatus: showStatus,
                status: status,
                awaitingInput: awaitingInput,
              ),
              _collapsible(
                colorScheme,
                animate: !reducedMotion,
                isExpanded: isExpanded,
              ),
              if (footer != null)
                SizedBox(width: double.infinity, child: footer),
            ],
          ),
        ),
      ),
    );
  }
}
