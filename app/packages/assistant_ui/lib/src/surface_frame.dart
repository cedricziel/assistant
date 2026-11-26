import 'package:material_ui/material_ui.dart';

import 'models/surface_record.dart';
import 'models/timeline_state.dart';
import 'theme/assistant_ui_spacing.dart';

/// The chrome around one agent-composed UI surface, hosted in the timeline
/// entry shell's body slot.
///
/// The frame owns no rendering of the surface: the app passes the rendered
/// tree as [content]. It owns what surrounds it:
///
/// - Without [content], or with [hasError], it shows [fallbackText] (with an
///   error affordance for [hasError]) instead of a broken tree.
/// - [pending] shows a progress bar while the surface is still being built.
/// - A [SurfaceStatus.submitted] surface reads `Submitted “<title>”` with a
///   "via form" marker. A surface that is active but not [answerable] is
///   labelled "Closed". Both show [content] read-only: pointer input, focus
///   and semantic actions are blocked, so no action fires.
/// - [awaitingInput] paints an attention accent. Hosts pass [entryState] to
///   the shell so it stays expanded.
///
/// Pure: constructor data only, no I/O.
class SurfaceFrame extends StatelessWidget {
  const SurfaceFrame({
    super.key,
    required this.title,
    required this.status,
    required this.answerable,
    required this.fallbackText,
    this.hasError = false,
    this.pending = false,
    this.awaitingInput = false,
    this.content,
  });

  final String title;
  final SurfaceStatus status;

  /// False once a later user message exists.
  final bool answerable;

  /// Plain text shown when there is nothing to render.
  final String fallbackText;

  /// The renderer could not build the surface; show [fallbackText].
  final bool hasError;

  /// The surface is still being built.
  final bool pending;

  /// The turn is waiting on the user's answer to this surface.
  final bool awaitingInput;

  /// The rendered surface.
  final Widget? content;

  bool get _submitted => status == SurfaceStatus.submitted;
  bool get _closed => status == SurfaceStatus.active && !answerable;
  bool get _readOnly => _submitted || _closed;

  /// The entry state the host gives the shell that contains this frame.
  EntryState get entryState {
    if (awaitingInput) return EntryState.awaitingInput;
    if (pending) return EntryState.active;
    return EntryState.complete;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final heading = _submitted ? 'Submitted “$title”' : title;

    return DecoratedBox(
      decoration: BoxDecoration(
        color: scheme.surfaceContainerLow,
        border: Border.all(color: scheme.outlineVariant),
        borderRadius: BorderRadius.circular(AssistantUiSpacing.md),
      ),
      position: DecorationPosition.background,
      child: DecoratedBox(
        // Painted over the card so the accent moves no layout.
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(AssistantUiSpacing.md),
          border: awaitingInput
              ? Border.all(color: scheme.primary, width: 1.5)
              : null,
        ),
        position: DecorationPosition.foreground,
        child: Padding(
          padding: const EdgeInsets.all(AssistantUiSpacing.md),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _header(theme, heading),
              const SizedBox(height: AssistantUiSpacing.sm),
              _body(theme),
            ],
          ),
        ),
      ),
    );
  }

  Widget _header(ThemeData theme, String heading) {
    final scheme = theme.colorScheme;
    final marker = _submitted
        ? 'via form'
        : _closed
        ? 'Closed'
        : awaitingInput
        ? 'Waiting for your answer'
        : null;

    return Row(
      children: [
        Expanded(
          child: Text(
            heading,
            style: theme.textTheme.titleSmall,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
          ),
        ),
        if (marker != null) ...[
          const SizedBox(width: AssistantUiSpacing.sm),
          DecoratedBox(
            decoration: BoxDecoration(
              color: awaitingInput
                  ? scheme.primaryContainer
                  : scheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(AssistantUiSpacing.sm),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(
                horizontal: AssistantUiSpacing.sm,
                vertical: 2,
              ),
              child: Text(
                marker,
                style: theme.textTheme.labelSmall?.copyWith(
                  color: awaitingInput
                      ? scheme.onPrimaryContainer
                      : scheme.onSurfaceVariant,
                ),
              ),
            ),
          ),
        ],
      ],
    );
  }

  Widget _body(ThemeData theme) {
    final scheme = theme.colorScheme;
    final tree = content;

    if (pending) {
      return const LinearProgressIndicator(minHeight: 2);
    }
    if (hasError || tree == null) {
      return Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (hasError) ...[
            Icon(Icons.error_outline, size: 18, color: scheme.error),
            const SizedBox(width: AssistantUiSpacing.sm),
          ],
          Expanded(
            child: Text(fallbackText, style: theme.textTheme.bodyMedium),
          ),
        ],
      );
    }
    if (!_readOnly) return tree;

    // A disabled scope: no pointer input, no focus and no semantic actions.
    return Opacity(
      opacity: 0.6,
      child: ExcludeFocus(child: IgnorePointer(child: tree)),
    );
  }
}
