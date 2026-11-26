import 'package:flutter/gestures.dart';
import 'package:material_ui/material_ui.dart';

/// A conversation as the list shows it.
class ConversationListItem {
  const ConversationListItem({
    required this.id,
    required this.title,
    required this.updatedAt,
    this.pinnedAt,
  });

  final String id;
  final String title;
  final DateTime updatedAt;

  /// When the conversation was pinned, or `null` when it is not pinned.
  final DateTime? pinnedAt;
}

/// A request to show the actions of one conversation row.
///
/// The view only reports where and how the request started; the caller picks
/// the presentation (a menu or a sheet).
class ConversationActionRequest {
  const ConversationActionRequest({
    required this.item,
    this.kind,
    this.position,
    this.anchorRect,
  });

  /// The row the request is for.
  final ConversationListItem item;

  /// The pointer that started the gesture, or `null` for keyboard activation.
  final PointerDeviceKind? kind;

  /// Global pointer position for a long-press or secondary tap, `null` for the
  /// overflow button.
  final Offset? position;

  /// Global rect of the overflow button, `null` for row gestures.
  final Rect? anchorRect;
}

/// Builds the optional trailing widget of one row.
typedef ConversationTrailingBuilder = Widget? Function(
  BuildContext context,
  ConversationListItem item,
);

/// A scrolling list of conversations with tap-to-open and swipe-to-delete.
///
/// The view owns no state and never navigates: a tap calls [onSelect] with the
/// conversation id. A swipe calls [onDelete], which the caller implements as
/// confirm-then-delete; the row is dismissed only when it resolves to `true`,
/// so a declined confirmation or a failed delete leaves the row in place.
///
/// [header] sits above the rows and stays visible for an empty list, where
/// [emptyState] fills the remaining space. Relative dates are computed against
/// [now], which defaults to the current time. The row whose id equals
/// [selectedId] is highlighted. When any item has a `pinnedAt`, a "Pinned"
/// header precedes the first pinned row; the caller supplies [items] already
/// ordered, pinned rows first.
///
/// Row actions are requested through [onRequestActions] on long-press, on
/// secondary tap and from a trailing overflow button shown while the row is
/// hovered, focused or selected. The only swipe is end-to-start delete.
class ConversationListView extends StatelessWidget {
  const ConversationListView({
    super.key,
    required this.items,
    required this.onSelect,
    required this.onDelete,
    this.header,
    this.emptyState,
    this.trailingBuilder,
    this.selectedId,
    this.onRequestActions,
    this.now,
  });

  final List<ConversationListItem> items;
  final ValueChanged<String> onSelect;
  final Future<bool> Function(String id) onDelete;
  final Widget? header;
  final Widget? emptyState;
  final ConversationTrailingBuilder? trailingBuilder;
  final String? selectedId;
  final ValueChanged<ConversationActionRequest>? onRequestActions;
  final DateTime? now;

  @override
  Widget build(BuildContext context) {
    final reference = now ?? DateTime.now();
    final scheme = Theme.of(context).colorScheme;
    final firstPinned = items.indexWhere((item) => item.pinnedAt != null);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ?header,
        Expanded(
          child: items.isEmpty
              ? emptyState ?? const SizedBox.shrink()
              : ListView.builder(
                  itemCount: items.length + (firstPinned < 0 ? 0 : 1),
                  itemBuilder: (context, index) {
                    if (firstPinned >= 0) {
                      if (index == firstPinned) {
                        return const _PinnedHeader();
                      }
                      if (index > firstPinned) index -= 1;
                    }
                    final item = items[index];
                    return Dismissible(
                      key: ValueKey(item.id),
                      direction: DismissDirection.endToStart,
                      background: Container(
                        alignment: Alignment.centerRight,
                        padding: const EdgeInsets.only(right: 16),
                        color: scheme.error,
                        child: Icon(Icons.delete, color: scheme.onError),
                      ),
                      confirmDismiss: (_) => onDelete(item.id),
                      child: _ConversationRow(
                        item: item,
                        selected: item.id == selectedId,
                        subtitle: _formatDate(reference, item.updatedAt),
                        trailing: trailingBuilder?.call(context, item),
                        onTap: () => onSelect(item.id),
                        onRequestActions: onRequestActions,
                      ),
                    );
                  },
                ),
        ),
      ],
    );
  }

  static String _formatDate(DateTime now, DateTime then) {
    final diff = now.difference(then);
    if (diff.inMinutes < 1) return 'Just now';
    if (diff.inHours < 1) return '${diff.inMinutes}m ago';
    if (diff.inDays < 1) return '${diff.inHours}h ago';
    return '${diff.inDays}d ago';
  }
}

class _PinnedHeader extends StatelessWidget {
  const _PinnedHeader();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
      child: Text(
        'Pinned',
        style: theme.textTheme.labelMedium?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

const _overflowLabel = 'Conversation actions';

class _ConversationRow extends StatefulWidget {
  const _ConversationRow({
    required this.item,
    required this.selected,
    required this.subtitle,
    required this.onTap,
    this.trailing,
    this.onRequestActions,
  });

  final ConversationListItem item;
  final bool selected;
  final String subtitle;
  final Widget? trailing;
  final VoidCallback onTap;
  final ValueChanged<ConversationActionRequest>? onRequestActions;

  @override
  State<_ConversationRow> createState() => _ConversationRowState();
}

class _ConversationRowState extends State<_ConversationRow> {
  final _buttonKey = GlobalKey();
  bool _hovered = false;
  bool _focused = false;

  /// Pointer kind of the last press on the row and on the overflow button.
  /// The button's kind is consumed on activation, so a keyboard activation
  /// (no preceding pointer-down) reports `null`.
  PointerDeviceKind? _rowKind;
  PointerDeviceKind? _buttonKind;

  void _request({PointerDeviceKind? kind, Offset? position, Rect? anchorRect}) {
    widget.onRequestActions?.call(
      ConversationActionRequest(
        item: widget.item,
        kind: kind,
        position: position,
        anchorRect: anchorRect,
      ),
    );
  }

  void _onOverflowPressed() {
    final kind = _buttonKind;
    _buttonKind = null;
    final box = _buttonKey.currentContext?.findRenderObject();
    final rect = box is RenderBox && box.hasSize
        ? box.localToGlobal(Offset.zero) & box.size
        : null;
    _request(kind: kind, anchorRect: rect);
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final canRequest = widget.onRequestActions != null;
    final showOverflow =
        canRequest && (_hovered || _focused || widget.selected);
    final trailing = widget.trailing;

    return MouseRegion(
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: Focus(
        canRequestFocus: false,
        skipTraversal: true,
        onFocusChange: (value) => setState(() => _focused = value),
        child: Listener(
          onPointerDown: (event) => _rowKind = event.kind,
          child: GestureDetector(
            behavior: HitTestBehavior.translucent,
            onLongPressStart: canRequest
                ? (details) =>
                      _request(kind: _rowKind, position: details.globalPosition)
                : null,
            onSecondaryTapUp: canRequest
                ? (details) =>
                      _request(kind: _rowKind, position: details.globalPosition)
                : null,
            child: ListTile(
              selected: widget.selected,
              selectedTileColor: scheme.secondaryContainer,
              selectedColor: scheme.onSecondaryContainer,
              title: Text(
                widget.item.title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              subtitle: Text(
                widget.subtitle,
                style: const TextStyle(fontSize: 11),
              ),
              trailing: trailing == null && !showOverflow
                  ? null
                  : Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        ?trailing,
                        if (showOverflow)
                          Listener(
                            onPointerDown: (event) => _buttonKind = event.kind,
                            child: Semantics(
                              label: _overflowLabel,
                              button: true,
                              onTap: _onOverflowPressed,
                              excludeSemantics: true,
                              child: Tooltip(
                                message: _overflowLabel,
                                child: IconButton(
                                  key: _buttonKey,
                                  icon: const Icon(Icons.more_horiz),
                                  onPressed: _onOverflowPressed,
                                ),
                              ),
                            ),
                          ),
                      ],
                    ),
              onTap: widget.onTap,
            ),
          ),
        ),
      ),
    );
  }
}
