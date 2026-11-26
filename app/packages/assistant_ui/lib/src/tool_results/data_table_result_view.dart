import 'dart:convert';

import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';
import '../theme/assistant_ui_spacing.dart';
import 'tool_result_renderer.dart';

/// One column of a `data_table` result.
class DataTableColumnModel {
  const DataTableColumnModel({
    required this.key,
    required this.label,
    this.type,
  });

  /// The row key this column reads.
  final String key;

  final String label;

  /// Optional hint such as `number`; numeric columns sort numerically and
  /// align to the end.
  final String? type;
}

/// The parsed `data_table` contract (`schemas/tool-results/data_table.schema.json`).
class DataTableModel {
  const DataTableModel({
    required this.columns,
    required this.rows,
    required this.truncated,
    required this.totalRows,
  });

  final List<DataTableColumnModel> columns;
  final List<Map<String, Object?>> rows;

  /// Whether [rows] is a prefix of the full result.
  final bool truncated;

  /// The row count before truncation.
  final int totalRows;
}

/// Renders `data_table` tool results with [DataTableResultView].
class DataTableResultRenderer extends ToolResultRenderer<DataTableModel> {
  const DataTableResultRenderer();

  @override
  String get kind => 'data_table';

  @override
  DataTableModel? parse(Map<String, Object?> data) {
    if (data['kind'] != kind) return null;
    final columns = data['columns'];
    final rows = data['rows'];
    final truncated = data['truncated'];
    final totalRows = data['total_rows'];
    if (columns is! List ||
        rows is! List ||
        truncated is! bool ||
        totalRows is! int ||
        totalRows < 0) {
      return null;
    }
    final parsedColumns = <DataTableColumnModel>[];
    for (final column in columns) {
      if (column is! Map) return null;
      final key = column['key'];
      final label = column['label'];
      final type = column['type'];
      if (key is! String || label is! String) return null;
      parsedColumns.add(
        DataTableColumnModel(
          key: key,
          label: label,
          type: type is String ? type : null,
        ),
      );
    }
    final parsedRows = <Map<String, Object?>>[];
    for (final row in rows) {
      if (row is! Map) return null;
      parsedRows.add(Map<String, Object?>.from(row));
    }
    return DataTableModel(
      columns: parsedColumns,
      rows: parsedRows,
      truncated: truncated,
      totalRows: totalRows,
    );
  }

  @override
  Widget summary(BuildContext context, DataTableModel model) {
    final count = model.rows.length;
    final text = switch (count) {
      0 => 'No rows',
      _ when model.truncated => '$count of ${model.totalRows} rows',
      1 => '1 row',
      _ => '$count rows',
    };
    return Text(
      text,
      style: Theme.of(context).textTheme.bodySmall,
      overflow: TextOverflow.ellipsis,
    );
  }

  @override
  Widget build(
    BuildContext context,
    DataTableModel model,
    ToolCallStatus status,
  ) => DataTableResultView(model: model);
}

/// A tool-result table: header from the columns, tap a header to sort by it
/// (tap again to reverse), horizontal scroll when the columns exceed the
/// width, and a notice when the rows were truncated.
///
/// Sorting is local widget state and never leaves the package.
class DataTableResultView extends StatefulWidget {
  const DataTableResultView({super.key, required this.model});

  final DataTableModel model;

  @override
  State<DataTableResultView> createState() => _DataTableResultViewState();
}

class _DataTableResultViewState extends State<DataTableResultView> {
  static const _maxHeight = 320.0;

  int? _sortColumn;
  bool _ascending = true;

  void _sort(int column, bool ascending) => setState(() {
    _sortColumn = column;
    _ascending = ascending;
  });

  List<Map<String, Object?>> get _rows {
    final rows = widget.model.rows;
    final index = _sortColumn;
    if (index == null) return rows;
    final column = widget.model.columns[index];
    final sign = _ascending ? 1 : -1;
    final order = {for (var i = 0; i < rows.length; i++) rows[i]: i};
    return [...rows]..sort((a, b) {
      final x = a[column.key];
      final y = b[column.key];
      // Missing values sort last in both directions.
      if (x == null || y == null) {
        return x == y ? 0 : (x == null ? 1 : -1);
      }
      final byValue = _compare(x, y, numeric: column.type == 'number');
      return byValue != 0 ? sign * byValue : order[a]!.compareTo(order[b]!);
    });
  }

  @override
  Widget build(BuildContext context) {
    final model = widget.model;
    final textTheme = Theme.of(context).textTheme;
    if (model.rows.isEmpty) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: AssistantUiSpacing.xs),
        child: Text('No rows', style: textTheme.bodySmall),
      );
    }
    final rows = _rows;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: _maxHeight),
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: SingleChildScrollView(
              child: DataTable(
                sortColumnIndex: _sortColumn,
                sortAscending: _ascending,
                headingRowHeight: 36,
                dataRowMinHeight: 32,
                dataRowMaxHeight: 40,
                columnSpacing: AssistantUiSpacing.lg,
                horizontalMargin: AssistantUiSpacing.sm,
                headingTextStyle: textTheme.labelMedium,
                dataTextStyle: textTheme.bodySmall,
                columns: [
                  for (final column in model.columns)
                    DataColumn(
                      label: Text(column.label),
                      numeric: column.type == 'number',
                      onSort: _sort,
                    ),
                ],
                rows: [
                  for (final row in rows)
                    DataRow(
                      cells: [
                        for (final column in model.columns)
                          DataCell(Text(_cellText(row[column.key]))),
                      ],
                    ),
                ],
              ),
            ),
          ),
        ),
        if (model.truncated)
          Padding(
            padding: const EdgeInsets.only(top: AssistantUiSpacing.xs),
            child: Text(
              'Showing ${model.rows.length} of ${model.totalRows} rows',
              style: textTheme.labelSmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ),
      ],
    );
  }
}

String _cellText(Object? value) => switch (value) {
  null => '—',
  String() => value,
  num() || bool() => '$value',
  _ => jsonEncode(value),
};

int _compare(Object x, Object y, {required bool numeric}) {
  if (x is num && y is num) return x.compareTo(y);
  if (numeric) {
    final a = num.tryParse('$x');
    final b = num.tryParse('$y');
    if (a != null && b != null) return a.compareTo(b);
  }
  return _cellText(x).toLowerCase().compareTo(_cellText(y).toLowerCase());
}
