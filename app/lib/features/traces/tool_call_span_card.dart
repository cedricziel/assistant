import 'dart:convert';

import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';

import 'span_classifier.dart';

/// Dedicated card for tool-call spans in the trace detail view (#265).
///
/// Surfaces tool name + status + duration in the header, and a two-pane
/// Params / Output body when expanded.
class ToolCallSpanCard extends StatefulWidget {
  const ToolCallSpanCard({
    super.key,
    required this.span,
    required this.totalMs,
  });

  final SpanEntryResponse span;
  final int totalMs;

  @override
  State<ToolCallSpanCard> createState() => _ToolCallSpanCardState();
}

class _ToolCallSpanCardState extends State<ToolCallSpanCard> {
  bool _expanded = false;
  bool _showAllAttributes = false;

  Map<String, Object?> get _attrs {
    final raw = widget.span.attributes?.value;
    if (raw is Map) {
      return raw.map((k, v) => MapEntry(k.toString(), v));
    }
    return const {};
  }

  String get _status {
    final s = _attrs['tool_status'];
    if (s is String && s.isNotEmpty) return s;
    return 'unknown';
  }

  String get _toolName => toolNameFromSpan(widget.span);

  String _formatDuration(int ms) {
    if (ms < 1000) return '${ms}ms';
    return '${(ms / 1000).toStringAsFixed(2)}s';
  }

  double get _widthFraction {
    if (widget.totalMs <= 0) return 1.0;
    return (widget.span.durationMs / widget.totalMs).clamp(0.02, 1.0);
  }

  ({IconData icon, Color colour}) _statusVisuals(ColorScheme scheme) {
    switch (_status) {
      case 'ok':
        return (icon: Icons.check_circle, colour: scheme.tertiary);
      case 'error':
        return (icon: Icons.error, colour: scheme.error);
      case 'denied':
        return (icon: Icons.block, colour: scheme.error.withAlpha(0xCC));
      default:
        return (icon: Icons.help_outline, colour: scheme.onSurfaceVariant);
    }
  }

  String _prettyJson(String raw) {
    try {
      final decoded = jsonDecode(raw);
      return const JsonEncoder.withIndent('  ').convert(decoded);
    } catch (_) {
      return raw;
    }
  }

  Widget _paneLabel(String label, ThemeData theme) {
    return Text(
      label,
      style: theme.textTheme.labelMedium?.copyWith(
        color: theme.colorScheme.onSurfaceVariant,
        fontWeight: FontWeight.w600,
      ),
    );
  }

  Widget _paneBody(String text, ThemeData theme, {Color? colour}) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: SelectableText(
        text,
        style: TextStyle(
          fontFamily: 'monospace',
          fontSize: 11,
          color: colour ?? theme.colorScheme.onSurface,
        ),
      ),
    );
  }

  Widget _paramsPane(ThemeData theme) {
    final raw = _attrs['tool_params'];
    final text = raw is String ? _prettyJson(raw) : '(no params)';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _paneLabel('Params', theme),
        const SizedBox(height: 4),
        _paneBody(text, theme),
      ],
    );
  }

  Widget _outputPane(ThemeData theme) {
    final scheme = theme.colorScheme;
    if (_status == 'error' || _status == 'denied') {
      final err = _attrs['tool_error'];
      final text = err is String ? err : '(no output)';
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _paneLabel('Output', theme),
          const SizedBox(height: 4),
          _paneBody(text, theme, colour: scheme.error),
        ],
      );
    }
    final obs = _attrs['tool_observation'];
    final text = obs is String ? _prettyJson(obs) : '(no output)';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _paneLabel('Output', theme),
        const SizedBox(height: 4),
        _paneBody(text, theme),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final visuals = _statusVisuals(scheme);

    return Card(
      margin: const EdgeInsets.only(bottom: 6),
      child: InkWell(
        onTap: () => setState(() => _expanded = !_expanded),
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Header row: icon + tool name + status pill + duration.
              Row(
                children: [
                  Icon(visuals.icon, color: visuals.colour, size: 18),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      _toolName,
                      style: const TextStyle(
                        fontFamily: 'monospace',
                        fontSize: 13,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: visuals.colour.withAlpha(0x33),
                      borderRadius: BorderRadius.circular(10),
                    ),
                    child: Text(
                      _status,
                      style: TextStyle(
                        fontSize: 11,
                        color: visuals.colour,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    _formatDuration(widget.span.durationMs),
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color: scheme.primary,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 6),
              // Duration bar.
              LayoutBuilder(
                builder: (_, constraints) => Stack(
                  children: [
                    Container(
                      height: 4,
                      width: double.infinity,
                      decoration: BoxDecoration(
                        color: scheme.outlineVariant,
                        borderRadius: BorderRadius.circular(2),
                      ),
                    ),
                    Container(
                      height: 4,
                      width: constraints.maxWidth * _widthFraction,
                      decoration: BoxDecoration(
                        color: scheme.primary,
                        borderRadius: BorderRadius.circular(2),
                      ),
                    ),
                  ],
                ),
              ),
              // Expanded body.
              if (_expanded) ...[
                const SizedBox(height: 10),
                const Divider(height: 1),
                const SizedBox(height: 8),
                LayoutBuilder(
                  builder: (_, constraints) {
                    final stack = constraints.maxWidth < 600;
                    if (stack) {
                      return Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          _paramsPane(theme),
                          const SizedBox(height: 10),
                          _outputPane(theme),
                        ],
                      );
                    }
                    return Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Expanded(child: _paramsPane(theme)),
                        const SizedBox(width: 12),
                        Expanded(child: _outputPane(theme)),
                      ],
                    );
                  },
                ),
                const SizedBox(height: 10),
                _ShowAllAttributesToggle(
                  expanded: _showAllAttributes,
                  onTap: () =>
                      setState(() => _showAllAttributes = !_showAllAttributes),
                ),
                if (_showAllAttributes) ...[
                  const SizedBox(height: 8),
                  const Divider(height: 1),
                  const SizedBox(height: 8),
                  for (final entry in _attrs.entries)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 2),
                      child: Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          SizedBox(
                            width: 160,
                            child: Text(
                              entry.key,
                              style: TextStyle(
                                fontSize: 11,
                                fontFamily: 'monospace',
                                color: scheme.onSurfaceVariant,
                              ),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          Expanded(
                            child: SelectableText(
                              '${entry.value}',
                              style: const TextStyle(
                                fontSize: 11,
                                fontFamily: 'monospace',
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                ],
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _ShowAllAttributesToggle extends StatelessWidget {
  const _ShowAllAttributesToggle({required this.expanded, required this.onTap});

  final bool expanded;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              expanded ? Icons.keyboard_arrow_up : Icons.keyboard_arrow_down,
              size: 16,
              color: scheme.onSurfaceVariant,
            ),
            const SizedBox(width: 4),
            Text(
              'Show all attributes',
              style: TextStyle(fontSize: 11, color: scheme.onSurfaceVariant),
            ),
          ],
        ),
      ),
    );
  }
}
