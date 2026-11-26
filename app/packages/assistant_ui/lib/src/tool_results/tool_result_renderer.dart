import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';

/// Renders one kind of structured tool result, such as a terminal run.
///
/// A renderer is pure: it reads only its parsed model, the theme and the
/// ambient scopes (`ThreadDensityScope`, `ChatMarkdownScope`).
abstract class ToolResultRenderer<T extends Object> {
  const ToolResultRenderer();

  /// The `data['kind']` tag this renderer handles.
  String get kind;

  /// Reads the typed model from a tool's `data` map, or returns `null` when
  /// the payload does not match the contract. Total by contract: it should not
  /// throw, and the registry treats a throw like `null`.
  T? parse(Map<String, Object?> data);

  /// The one-line form shown at compact density, for example
  /// `exit 2 · 14 lines stderr`.
  Widget summary(BuildContext context, T model);

  /// The full view.
  Widget build(BuildContext context, T model, ToolCallStatus status);
}
