import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';

/// Builds a custom view for [record], or returns `null` to use the generic
/// fallback.
typedef ToolResultBuilder = Widget? Function(
  BuildContext context,
  ToolCallRecord record,
);

/// Supplies the function `ToolCallBody` calls to render a tool call's details.
///
/// The scope carries one function and knows nothing about registries, kinds or
/// parsing: its provider owns how a record resolves to a widget. Without an
/// ancestor scope [of] returns `null` and the generic view renders.
class ToolResultRendererScope extends InheritedWidget {
  const ToolResultRendererScope({
    super.key,
    required this.resolve,
    required super.child,
  });

  final ToolResultBuilder resolve;

  /// The resolver of the nearest scope, or `null` when there is none.
  /// Rebuilds the caller when the resolver changes.
  static ToolResultBuilder? of(BuildContext context) => context
      .dependOnInheritedWidgetOfExactType<ToolResultRendererScope>()
      ?.resolve;

  @override
  bool updateShouldNotify(ToolResultRendererScope oldWidget) =>
      resolve != oldWidget.resolve;
}
