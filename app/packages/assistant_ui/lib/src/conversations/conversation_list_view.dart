import 'package:flutter/gestures.dart';
import 'package:material_ui/material_ui.dart';

/// A conversation as the list shows it.
class ConversationListItem {
  const ConversationListItem({
    required this.id,
    required this.title,
    required this.updatedAt,
    this.pinnedAt,
    this.archivedAt,
  });

  final String id;
  final String title;
  final DateTime updatedAt;

  /// When the conversation was pinned, or `null` when it is not pinned.
  final DateTime? pinnedAt;

  /// When the conversation was archived, or `null` when it is not archived.
  final DateTime? archivedAt;
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
///
/// [archivedSection] is a slot below the rows, normally a
/// [ConversationArchivedSection]. It scrolls with the rows and stays visible
/// when [items] is empty.
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
    this.archivedSection,
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
  final Widget? archivedSection;
  final DateTime? now;

  @override
  Widget build(BuildContext context) {
    final reference = now ?? DateTime.now();
    final slot = archivedSection;
    final firstPinned = items.indexWhere((item) => item.pinnedAt != null);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ?header,
        Expanded(
          child: items.isEmpty
              ? _buildEmpty()
              : ListView.builder(
                  itemCount:
                      items.length +
                      (firstPinned < 0 ? 0 : 1) +
                      (slot == null ? 0 : 1),
                  itemBuilder: (context, index) {
                    if (firstPinned >= 0) {
                      if (index == firstPinned) {
                        return const _PinnedHeader();
                      }
                      if (index > firstPinned) index -= 1;
                    }
                    if (index >= items.length) return slot;
                    return _DismissibleRow(
                      item: items[index],
                      selected: items[index].id == selectedId,
                      now: reference,
                      trailing: trailingBuilder?.call(context, items[index]),
                      onSelect: onSelect,
                      onDelete: onDelete,
                      onRequestActions: onRequestActions,
                    );
                  },
                ),
        ),
      ],
    );
  }

  /// The empty state, with the archived slot below it when there is one.
  Widget _buildEmpty() {
    final slot = archivedSection;
    final empty = emptyState;
    if (slot == null) return empty ?? const SizedBox.shrink();
    return ListView(
      children: [
        if (empty != null) SizedBox(height: 160, child: empty),
        slot,
      ],
    );
  }
}

/// One row wrapped in its swipe-to-delete [Dismissible].
class _DismissibleRow extends StatelessWidget {
  const _DismissibleRow({
    required this.item,
    required this.selected,
    required this.now,
    required this.onSelect,
    required this.onDelete,
    this.trailing,
    this.onRequestActions,
  });

  final ConversationListItem item;
  final bool selected;
  final DateTime now;
  final Widget? trailing;
  final ValueChanged<String> onSelect;
  final Future<bool> Function(String id) onDelete;
  final ValueChanged<ConversationActionRequest>? onRequestActions;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
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
        selected: selected,
        subtitle: _formatDate(now, item.updatedAt),
        trailing: trailing,
        onTap: () => onSelect(item.id),
        onRequestActions: onRequestActions,
      ),
    );
  }
}

String _formatDate(DateTime now, DateTime then) {
  final diff = now.difference(then);
  if (diff.inMinutes < 1) return 'Just now';
  if (diff.inHours < 1) return '${diff.inMinutes}m ago';
  if (diff.inDays < 1) return '${diff.inHours}h ago';
  return '${diff.inDays}d ago';
}

/// The collapsible "Archived" section for [ConversationListView.archivedSection].
///
/// Pure: it owns no state and does no I/O. The caller keeps [expanded], loads
/// [items] when [onExpandedChanged] reports `true`, and reports [loading] and
/// [error]. Its rows behave like the list's own: tap selects, swipe deletes
/// through [onDelete], and the gestures emit [onRequestActions].
class ConversationArchivedSection extends StatelessWidget {
  const ConversationArchivedSection({
    super.key,
    required this.expanded,
    required this.onExpandedChanged,
    required this.items,
    required this.onSelect,
    required this.onDelete,
    this.loading = false,
    this.error,
    this.onRetry,
    this.selectedId,
    this.onRequestActions,
    this.now,
  });

  final bool expanded;
  final ValueChanged<bool> onExpandedChanged;

  /// The archived conversations, most recently archived first.
  final List<ConversationListItem> items;
  final ValueChanged<String> onSelect;
  final Future<bool> Function(String id) onDelete;

  /// A fetch is in flight. Shown as a spinner only while [items] is empty, so
  /// a refetch does not flash the rows away.
  final bool loading;

  /// Why the last fetch failed, shown with a Retry button.
  final String? error;
  final VoidCallback? onRetry;
  final String? selectedId;
  final ValueChanged<ConversationActionRequest>? onRequestActions;
  final DateTime? now;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final reference = now ?? DateTime.now();
    final muted = TextStyle(color: theme.colorScheme.onSurfaceVariant);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Semantics(
          button: true,
          expanded: expanded,
          excludeSemantics: true,
          label: 'Archived',
          onTap: () => onExpandedChanged(!expanded),
          child: InkWell(
            onTap: () => onExpandedChanged(!expanded),
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
              child: Row(
                children: [
                  Expanded(
                    child: Text(
                      'Archived',
                      style: theme.textTheme.labelMedium?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  Icon(
                    expanded ? Icons.expand_less : Icons.expand_more,
                    size: 18,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ],
              ),
            ),
          ),
        ),
        if (expanded)
          if (error != null && items.isEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Row(
                children: [
                  Expanded(child: Text(error!, style: muted)),
                  if (onRetry != null)
                    TextButton(onPressed: onRetry, child: const Text('Retry')),
                ],
              ),
            )
          else if (items.isEmpty && loading)
            const Padding(
              padding: EdgeInsets.all(16),
              child: Center(child: CircularProgressIndicator.adaptive()),
            )
          else if (items.isEmpty)
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
              child: Text('No archived conversations', style: muted),
            )
          else
            for (final item in items)
              _DismissibleRow(
                item: item,
                selected: item.id == selectedId,
                now: reference,
                onSelect: onSelect,
                onDelete: onDelete,
                onRequestActions: onRequestActions,
              ),
      ],
    );
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
