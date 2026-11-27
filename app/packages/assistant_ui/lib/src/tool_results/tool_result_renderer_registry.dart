import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';
import '../models/timeline_state.dart';
import '../scopes/thread_density_scope.dart';
import 'code_diff_result_view.dart';
import 'code_file_result_view.dart';
import 'compact_expandable.dart';
import 'data_table_result_view.dart';
import 'link_preview_result_view.dart';
import 'plan_result_view.dart';
import 'sources_result_view.dart';
import 'terminal_result_view.dart';
import 'tool_result_renderer.dart';

/// The renderers the app installs by default, one per structured result kind.
List<ToolResultRenderer<Object>> defaultToolResultRenderers() => const [
  CodeDiffRenderer(),
  CodeFileRenderer(),
  DataTableResultRenderer(),
  LinkPreviewRenderer(),
  PlanResultRenderer(),
  SourcesRenderer(),
  TerminalRenderer(),
];

/// Resolves a tool-call record to the widget that shows its result, or `null`
/// when the generic view applies.
///
/// [build] matches `ToolResultBuilder`, so the app installs it with
/// `ToolResultRendererScope(resolve: registry.build)`.
class ToolResultRendererRegistry {
  const ToolResultRendererRegistry({
    this.byKind = const [],
    this.byToolName = const {},
  });

  /// Renderers chosen by the `kind` tag in a record's data.
  final List<ToolResultRenderer<Object>> byKind;

  /// Renderers chosen by exact tool name; they win over [byKind].
  final Map<String, ToolResultRenderer<Object>> byToolName;

  /// Resolves, in order: the renderer for the record's tool name, then the
  /// renderer for `data['kind']`. Returns `null` when none applies, when its
  /// `parse` yields `null` or throws, or when the status calls for the generic
  /// view (`pending`, `denied`, `cancelled`).
  ///
  /// A record that carries an MCP App is always left to the app's frame.
  ///
  /// At [TimelineDensity.compact] the result is the renderer's one-line
  /// summary with a toggle to the full view; otherwise it is the full view.
  Widget? build(BuildContext context, ToolCallRecord record) {
    if (record.app != null) return null;
    if (record.status != ToolCallStatus.ok &&
        record.status != ToolCallStatus.error) {
      return null;
    }
    final data = record.data;
    if (data == null) return null;
    final renderer = byToolName[record.toolName] ?? _byKind(data['kind']);
    if (renderer == null) return null;
    return _render(context, renderer, data, record.status);
  }

  ToolResultRenderer<Object>? _byKind(Object? kind) {
    if (kind is! String) return null;
    for (final renderer in byKind) {
      if (renderer.kind == kind) return renderer;
    }
    return null;
  }
}

Widget? _render<T extends Object>(
  BuildContext context,
  ToolResultRenderer<T> renderer,
  Map<String, Object?> data,
  ToolCallStatus status,
) {
  final T? model;
  try {
    model = renderer.parse(data);
  } on Object {
    return null;
  }
  if (model == null) return null;
  final full = renderer.build(context, model, status);
  if (ThreadDensityScope.of(context) != TimelineDensity.compact) return full;
  return CompactExpandable(
    summary: renderer.summary(context, model),
    expanded: full,
  );
}
