import 'dart:async';

import 'package:material_ui/material_ui.dart';

import '../highlight/highlight_code.dart';
import '../scopes/chat_markdown_scope.dart';
import '../theme/assistant_ui_spacing.dart';
import '../theme/code_theme.dart';

/// A labelled, copyable block of code.
///
/// The header shows the fence [language] as written (`text` when null) and,
/// when a copy callback is available, an always-visible copy button. The header
/// is excluded from text selection, so selecting across the block copies only
/// the code. The body scrolls horizontally and is painted with the ambient
/// [CodeTheme]'s own background.
///
/// Activating copy calls the callback with the exact [code], then shows
/// "Copied" for two seconds. The callback is [onCopy] when given, otherwise the
/// nearest `ChatMarkdownScope.onCopyCode`. The package never touches the
/// clipboard itself; without either callback no button renders.
///
/// Code is highlighted with [highlightCode] when [highlight] is true and
/// [language] names a known grammar; otherwise it renders as plain monospace.
class CodeBlock extends StatefulWidget {
  const CodeBlock({
    super.key,
    required this.code,
    this.language,
    this.onCopy,
    this.highlight = true,
  });

  final String code;

  /// The fence language as written, or null for plain `text`.
  final String? language;

  /// Receives [code] when the copy button is activated.
  final ValueChanged<String>? onCopy;

  /// Whether to syntax-highlight [code].
  final bool highlight;

  @override
  State<CodeBlock> createState() => _CodeBlockState();
}

class _CodeBlockState extends State<CodeBlock> {
  static const _copiedDuration = Duration(seconds: 2);
  static const _monospace = TextStyle(
    fontFamily: 'monospace',
    fontFamilyFallback: ['Menlo', 'Consolas', 'Roboto Mono', 'Courier New'],
    fontSize: 13,
    height: 1.5,
  );

  Timer? _revert;
  bool _copied = false;

  @override
  void dispose() {
    _revert?.cancel();
    super.dispose();
  }

  void _copy(ValueChanged<String> onCopy) {
    onCopy(widget.code);
    _revert?.cancel();
    setState(() => _copied = true);
    _revert = Timer(_copiedDuration, () {
      if (mounted) setState(() => _copied = false);
    });
  }

  @override
  Widget build(BuildContext context) {
    final brightness = Theme.of(context).brightness;
    final codeTheme = brightness == Brightness.dark
        ? CodeTheme.dark
        : CodeTheme.light;
    final scheme = Theme.of(context).colorScheme;
    final language = widget.language?.trim();
    final label = language == null || language.isEmpty ? 'text' : language;
    final onCopy = widget.onCopy ?? ChatMarkdownScope.of(context)?.onCopyCode;

    final tokens = widget.highlight
        ? highlightCode(widget.code, widget.language)
        : highlightCode(widget.code, null);

    return ClipRRect(
      borderRadius: BorderRadius.circular(8),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: codeTheme.background,
          border: Border.all(color: scheme.outlineVariant),
          borderRadius: BorderRadius.circular(8),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SelectionContainer.disabled(
              child: Padding(
                padding: const EdgeInsets.only(left: AssistantUiSpacing.md),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        label,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: _monospace.copyWith(
                          fontSize: 12,
                          color: codeTheme.foreground,
                        ),
                      ),
                    ),
                    if (onCopy != null)
                      _CopyButton(
                        copied: _copied,
                        color: codeTheme.foreground,
                        onPressed: () => _copy(onCopy),
                      ),
                  ],
                ),
              ),
            ),
            SingleChildScrollView(
              key: const Key('code_block_body'),
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.fromLTRB(
                AssistantUiSpacing.md,
                AssistantUiSpacing.xs,
                AssistantUiSpacing.md,
                AssistantUiSpacing.md,
              ),
              child: Text.rich(
                TextSpan(
                  style: _monospace.copyWith(color: codeTheme.foreground),
                  children: [
                    for (final token in tokens)
                      TextSpan(
                        text: token.text,
                        style: token.scope == null
                            ? null
                            : TextStyle(color: codeTheme.colorFor(token.scope)),
                      ),
                  ],
                ),
                softWrap: false,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _CopyButton extends StatelessWidget {
  const _CopyButton({
    required this.copied,
    required this.color,
    required this.onPressed,
  });

  final bool copied;
  final Color color;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final text = copied ? 'Copied' : 'Copy code';
    return Tooltip(
      message: text,
      excludeFromSemantics: true,
      child: Semantics(
        button: true,
        label: text,
        excludeSemantics: true,
        child: InkWell(
          onTap: onPressed,
          borderRadius: BorderRadius.circular(6),
          child: ConstrainedBox(
            constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
            child: Padding(
              padding: const EdgeInsets.symmetric(
                horizontal: AssistantUiSpacing.sm,
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(
                    copied ? Icons.check : Icons.content_copy,
                    size: 14,
                    color: color,
                  ),
                  if (copied) ...[
                    const SizedBox(width: AssistantUiSpacing.xs),
                    Text(
                      'Copied',
                      style: TextStyle(fontSize: 12, color: color),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
