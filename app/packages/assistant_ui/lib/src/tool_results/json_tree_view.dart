import 'dart:convert';

import 'package:material_ui/material_ui.dart';

import '../theme/assistant_ui_spacing.dart';

/// A read-only, collapsible tree of structured tool data.
///
/// Objects and arrays are nodes a tap opens or closes, and show their size
/// while closed. Scalars show inline, coloured by type. [data] is the root
/// object; its entries are always visible, and nested containers open to
/// [initiallyExpandedDepth] levels (1 shows only the root's entries).
///
/// Children of a closed node are not built, so a large payload costs only what
/// the user opens. The open state is local and resets when the widget leaves
/// the tree.
class JsonTreeView extends StatelessWidget {
  const JsonTreeView({
    super.key,
    required this.data,
    this.initiallyExpandedDepth = 1,
  });

  final Map<String, Object?> data;

  /// How many levels from the root start open. The root counts as level 1.
  final int initiallyExpandedDepth;

  @override
  Widget build(BuildContext context) {
    return DefaultTextStyle.merge(
      style: const TextStyle(fontSize: 10, fontFamily: 'monospace'),
      child: data.isEmpty
          ? const _Leaf(label: null, value: {}, depth: 0)
          : _Children(
              value: data,
              depth: 0,
              initiallyExpandedDepth: initiallyExpandedDepth,
            ),
    );
  }
}

const double _indent = AssistantUiSpacing.md;

Iterable<MapEntry<String, Object?>> _entries(Object? value) => switch (value) {
  final Map<dynamic, dynamic> m => m.entries.map(
    (e) => MapEntry(e.key.toString(), e.value),
  ),
  final List<dynamic> l => l.indexed.map((e) => MapEntry('${e.$1}', e.$2)),
  _ => const [],
};

bool _isContainer(Object? value) =>
    (value is Map && value.isNotEmpty) || (value is List && value.isNotEmpty);

class _Children extends StatelessWidget {
  const _Children({
    required this.value,
    required this.depth,
    required this.initiallyExpandedDepth,
  });

  final Object? value;
  final int depth;
  final int initiallyExpandedDepth;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    mainAxisSize: MainAxisSize.min,
    children: [
      for (final entry in _entries(value))
        if (_isContainer(entry.value))
          _Node(
            label: entry.key,
            value: entry.value,
            depth: depth,
            initiallyExpandedDepth: initiallyExpandedDepth,
          )
        else
          _Leaf(label: entry.key, value: entry.value, depth: depth),
    ],
  );
}

class _Node extends StatefulWidget {
  const _Node({
    required this.label,
    required this.value,
    required this.depth,
    required this.initiallyExpandedDepth,
  });

  final String label;
  final Object? value;
  final int depth;
  final int initiallyExpandedDepth;

  @override
  State<_Node> createState() => _NodeState();
}

class _NodeState extends State<_Node> {
  late bool _open = widget.depth + 1 < widget.initiallyExpandedDepth;

  void _toggle() => setState(() => _open = !_open);

  String get _summary => widget.value is List
      ? '[${(widget.value as List).length}]'
      : '{${(widget.value as Map).length}}';

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Semantics(
          container: true,
          button: true,
          expanded: _open,
          label: '${widget.label} $_summary',
          onTap: _toggle,
          excludeSemantics: true,
          child: InkWell(
            excludeFromSemantics: true,
            onTap: _toggle,
            child: Padding(
              padding: EdgeInsets.only(left: widget.depth * _indent),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    _open ? Icons.expand_more : Icons.chevron_right,
                    size: 14,
                    color: colors.onSurfaceVariant,
                  ),
                  Text(widget.label, style: TextStyle(color: colors.primary)),
                  const SizedBox(width: AssistantUiSpacing.xs),
                  Text(
                    _summary,
                    style: TextStyle(color: colors.onSurfaceVariant),
                  ),
                ],
              ),
            ),
          ),
        ),
        if (_open)
          _Children(
            value: widget.value,
            depth: widget.depth + 1,
            initiallyExpandedDepth: widget.initiallyExpandedDepth,
          ),
      ],
    );
  }
}

class _Leaf extends StatelessWidget {
  const _Leaf({required this.label, required this.value, required this.depth});

  final String? label;
  final Object? value;
  final int depth;

  Color _color(ColorScheme colors) => switch (value) {
    String() => colors.tertiary,
    num() => colors.secondary,
    _ => colors.onSurface,
  };

  String get _text => switch (value) {
    String() => jsonEncode(value),
    Map() => '{}',
    List() => '[]',
    _ => '$value',
  };

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Padding(
      // Aligns with node labels, which follow a 14px chevron.
      padding: EdgeInsets.only(left: depth * _indent + 14),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (label != null) ...[
            Text(label!, style: TextStyle(color: colors.primary)),
            const SizedBox(width: AssistantUiSpacing.xs),
          ],
          Flexible(
            child: SelectableText(
              _text,
              style: TextStyle(color: _color(colors)),
            ),
          ),
        ],
      ),
    );
  }
}
