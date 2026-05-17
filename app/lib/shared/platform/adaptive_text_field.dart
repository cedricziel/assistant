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
    this.placeholder,
    this.onChanged,
    this.onSubmitted,
    this.keyboardType,
    this.obscureText = false,
    this.autofocus = false,
    this.textInputAction,
    this.prefix,
  });

  final TextEditingController? controller;
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

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch) {
      return CupertinoTextField(
        controller: controller,
        placeholder: placeholder,
        onChanged: onChanged,
        onSubmitted: onSubmitted,
        keyboardType: keyboardType,
        obscureText: obscureText,
        autofocus: autofocus,
        textInputAction: textInputAction,
        prefix: prefix,
      );
    }
    return TextField(
      controller: controller,
      decoration: (placeholder != null || prefix != null)
          ? InputDecoration(hintText: placeholder, prefixIcon: prefix)
          : null,
      onChanged: onChanged,
      onSubmitted: onSubmitted,
      keyboardType: keyboardType,
      obscureText: obscureText,
      autofocus: autofocus,
      textInputAction: textInputAction,
    );
  }
}
