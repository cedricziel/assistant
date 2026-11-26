import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter/services.dart' show HapticFeedback;

import 'adaptive_text_field.dart';
import 'platform.dart';

/// Shows a confirmation dialog styled for the current platform.
///
/// Returns `true` when the user taps the confirm action, `false` otherwise.
Future<bool> showAdaptiveConfirmDialog({
  required BuildContext context,
  required String title,
  required String content,
  required String confirmLabel,
  bool isDestructive = false,
}) async {
  if (isAppleTouch) {
    if (isDestructive) HapticFeedback.mediumImpact();
    final result = await showCupertinoDialog<bool>(
      context: context,
      builder: (ctx) => CupertinoAlertDialog(
        title: Text(title),
        content: Text(content),
        actions: [
          CupertinoDialogAction(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Cancel'),
          ),
          CupertinoDialogAction(
            isDestructiveAction: isDestructive,
            onPressed: () => Navigator.of(ctx).pop(true),
            child: Text(confirmLabel),
          ),
        ],
      ),
    );
    return result ?? false;
  }

  final result = await showDialog<bool>(
    context: context,
    builder: (ctx) => AlertDialog(
      title: Text(title),
      content: Text(content),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(ctx).pop(false),
          child: const Text('Cancel'),
        ),
        TextButton(
          onPressed: () => Navigator.of(ctx).pop(true),
          style: isDestructive
              ? TextButton.styleFrom(
                  foregroundColor: Theme.of(context).colorScheme.error,
                )
              : null,
          child: Text(confirmLabel),
        ),
      ],
    ),
  );
  return result ?? false;
}

/// Shows a single-line text input dialog styled for the current platform.
///
/// The field is prefilled with [initialText], fully selected and focused.
/// Confirm stays disabled while the trimmed text is empty or longer than
/// [maxLength]. The length is counted in Unicode scalar values
/// (`text.runes.length`), the unit the server validates, because Flutter's
/// own `maxLength` counts grapheme clusters.
///
/// Returns the trimmed text on confirm (button or Enter) and `null` when the
/// dialog is cancelled or dismissed (Cancel, Esc, barrier tap).
Future<String?> showAdaptiveTextInputDialog({
  required BuildContext context,
  required String title,
  required String confirmLabel,
  String initialText = '',
  String? placeholder,
  int maxLength = 200,
}) {
  Widget builder(BuildContext ctx) => _TextInputDialog(
    title: title,
    confirmLabel: confirmLabel,
    initialText: initialText,
    placeholder: placeholder,
    maxLength: maxLength,
  );

  if (isAppleTouch) {
    return showCupertinoDialog<String>(context: context, builder: builder);
  }
  return showDialog<String>(context: context, builder: builder);
}

class _TextInputDialog extends StatefulWidget {
  const _TextInputDialog({
    required this.title,
    required this.confirmLabel,
    required this.initialText,
    required this.placeholder,
    required this.maxLength,
  });

  final String title;
  final String confirmLabel;
  final String initialText;
  final String? placeholder;
  final int maxLength;

  @override
  State<_TextInputDialog> createState() => _TextInputDialogState();
}

class _TextInputDialogState extends State<_TextInputDialog> {
  late final TextEditingController _controller;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.initialText)
      ..selection = TextSelection(
        baseOffset: 0,
        extentOffset: widget.initialText.length,
      );
    _controller.addListener(_onChanged);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _onChanged() => setState(() {});

  int get _runeCount => _controller.text.runes.length;

  bool get _isValid =>
      _controller.text.trim().isNotEmpty && _runeCount <= widget.maxLength;

  void _confirm() {
    if (!_isValid) return;
    Navigator.of(context).pop(_controller.text.trim());
  }

  void _cancel() => Navigator.of(context).pop();

  @override
  Widget build(BuildContext context) {
    final overLimit = _runeCount > widget.maxLength;
    final counterColor = overLimit
        ? Theme.of(context).colorScheme.error
        : Theme.of(context).colorScheme.onSurfaceVariant;
    final content = Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const SizedBox(height: 12),
        AdaptiveTextField(
          controller: _controller,
          placeholder: widget.placeholder,
          autofocus: true,
          textInputAction: TextInputAction.done,
          onSubmitted: (_) => _confirm(),
        ),
        const SizedBox(height: 6),
        Align(
          alignment: Alignment.centerRight,
          child: Text(
            '$_runeCount/${widget.maxLength}',
            style: TextStyle(fontSize: 12, color: counterColor),
          ),
        ),
      ],
    );

    if (isAppleTouch) {
      return CupertinoAlertDialog(
        title: Text(widget.title),
        content: content,
        actions: [
          CupertinoDialogAction(
            onPressed: _cancel,
            child: const Text('Cancel'),
          ),
          CupertinoDialogAction(
            isDefaultAction: true,
            onPressed: _isValid ? _confirm : null,
            child: Text(widget.confirmLabel),
          ),
        ],
      );
    }

    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(width: 400, child: content),
      actions: [
        TextButton(onPressed: _cancel, child: const Text('Cancel')),
        TextButton(
          onPressed: _isValid ? _confirm : null,
          child: Text(widget.confirmLabel),
        ),
      ],
    );
  }
}
