import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// Single-line text input that renders [CupertinoTextField] on Apple touch
/// platforms and Material [TextField] on Material / macOS targets.
///
/// API exposes the common subset of both: [controller], [placeholder],
/// [onChanged], [onSubmitted], [keyboardType], [obscureText], [autofocus],
/// and [textInputAction]. The Material path maps [placeholder] to
/// `InputDecoration.hintText`.
class AdaptiveTextField extends StatelessWidget {
  const AdaptiveTextField({
    super.key,
    this.controller,
    this.focusNode,
    this.placeholder,
    this.onChanged,
    this.onSubmitted,
    this.keyboardType,
    this.obscureText = false,
    this.autofocus = false,
    this.textInputAction,
    this.prefix,
    this.minLines,
    this.maxLines = 1,
    this.cursorColor,
    this.contentPadding,
    this.enabled,
  });

  final TextEditingController? controller;
  final FocusNode? focusNode;
  final String? placeholder;
  final ValueChanged<String>? onChanged;
  final ValueChanged<String>? onSubmitted;
  final TextInputType? keyboardType;
  final bool obscureText;
  final bool autofocus;
  final TextInputAction? textInputAction;

  /// Optional leading widget rendered inside the field. Maps to
  /// `CupertinoTextField.prefix` on iOS and `InputDecoration.prefixIcon`
  /// on Material. Commonly an [Icon] used for search affordances.
  final Widget? prefix;

  /// Multi-line support. `maxLines = 1` (the default) renders a single
  /// line; pass `null` for unbounded growth.
  final int? minLines;
  final int? maxLines;

  /// Caret colour. Maps directly on both platforms.
  final Color? cursorColor;

  /// Inner padding. Maps to `CupertinoTextField.padding` on iOS and to
  /// `InputDecoration.contentPadding` on Material.
  final EdgeInsetsGeometry? contentPadding;

  /// Whether the field accepts input. `null` (the default) means enabled.
  final bool? enabled;

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch) {
      return CupertinoTextField(
        controller: controller,
        focusNode: focusNode,
        placeholder: placeholder,
        onChanged: onChanged,
        onSubmitted: onSubmitted,
        keyboardType: keyboardType,
        obscureText: obscureText,
        autofocus: autofocus,
        textInputAction: textInputAction,
        prefix: prefix,
        minLines: minLines,
        maxLines: maxLines,
        cursorColor: cursorColor,
        enabled: enabled ?? true,
        padding:
            contentPadding ??
            const EdgeInsets.symmetric(horizontal: 6, vertical: 7),
      );
    }
    return TextField(
      controller: controller,
      focusNode: focusNode,
      decoration:
          (placeholder != null || prefix != null || contentPadding != null)
          ? InputDecoration(
              hintText: placeholder,
              prefixIcon: prefix,
              contentPadding: contentPadding,
            )
          : null,
      onChanged: onChanged,
      onSubmitted: onSubmitted,
      keyboardType: keyboardType,
      obscureText: obscureText,
      autofocus: autofocus,
      textInputAction: textInputAction,
      minLines: minLines,
      maxLines: maxLines,
      cursorColor: cursorColor,
      enabled: enabled,
    );
  }
}
