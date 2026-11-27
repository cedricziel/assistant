import 'package:material_ui/material_ui.dart';

/// Colour treatment of a [MessageActionButton].
enum MessageActionTone {
  /// Quiet, the default for every action.
  muted,

  /// Destructive or error-recovery actions such as Retry.
  danger,
}

/// The standard action of a [MessageActions] row: a small icon and a text
/// label.
///
/// The [label] is shown and is also the semantics label. [selected] marks a
/// toggled action (for example a chosen rating) and is exposed to assistive
/// technology.
class MessageActionButton extends StatelessWidget {
  const MessageActionButton({
    super.key,
    required this.icon,
    required this.label,
    required this.onPressed,
    this.tone = MessageActionTone.muted,
    this.selected = false,
  });

  final IconData icon;
  final String label;

  /// Null disables the button.
  final VoidCallback? onPressed;
  final MessageActionTone tone;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final color = switch ((tone, selected)) {
      (MessageActionTone.danger, _) => scheme.error,
      (_, true) => scheme.primary,
      _ => scheme.onSurfaceVariant,
    };
    return Semantics(
      button: true,
      selected: selected,
      label: label,
      onTap: onPressed,
      excludeSemantics: true,
      child: InkWell(
        onTap: onPressed,
        borderRadius: BorderRadius.circular(4),
        // At least 24x24 (WCAG 2.2 SC 2.5.8), however small the label.
        child: ConstrainedBox(
          constraints: const BoxConstraints(minWidth: 24, minHeight: 24),
          child: Center(
            widthFactor: 1,
            heightFactor: 1,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(icon, size: 14, color: color),
                  const SizedBox(width: 4),
                  Text(label, style: TextStyle(fontSize: 12, color: color)),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// A row of caller-supplied message actions.
///
/// Lays out [leading], then [actions] in the order given, then [trailing]. It
/// never decides which actions apply to a message; the caller does. It renders
/// nothing when all three are empty.
class MessageActions extends StatelessWidget {
  const MessageActions({
    super.key,
    required this.actions,
    this.leading,
    this.trailing = const [],
    this.alignment = WrapAlignment.start,
  });

  final List<Widget> actions;

  /// Rendered before [actions], for example a branch picker.
  final Widget? leading;

  /// Rendered after [actions] in order, for example timing and feedback.
  final List<Widget> trailing;

  /// Horizontal alignment of the row, which follows the message's side.
  final WrapAlignment alignment;

  @override
  Widget build(BuildContext context) {
    final children = [?leading, ...actions, ...trailing];
    if (children.isEmpty) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.only(top: 4),
      child: Wrap(
        alignment: alignment,
        spacing: 16,
        runSpacing: 4,
        children: children,
      ),
    );
  }
}
