@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

const _nested = <String, Object?>{
  'pid': 4182,
  'status': 'running',
  'exit_code': null,
  'truncated': false,
  'env': {
    'PATH': '/usr/bin',
    'limits': {'cpu': 2, 'memory_mb': 512},
  },
  'tags': ['build', 'ci'],
};

Widget _padded(Widget child) =>
    Padding(padding: const EdgeInsets.all(12), child: child);

void main() {
  final cases = <String, ({Widget widget, Size size})>{
    'json_tree_view_collapsed': (
      widget: _padded(const JsonTreeView(data: _nested)),
      size: const Size(340, 150),
    ),
    'json_tree_view_expanded': (
      widget: _padded(
        const JsonTreeView(data: _nested, initiallyExpandedDepth: 3),
      ),
      size: const Size(340, 230),
    ),
    'generic_tool_result_view_data': (
      widget: _padded(
        GenericToolResultView(
          record: ToolCallRecord(
            toolName: 'file-glob',
            status: ToolCallStatus.ok,
            arguments: const {'pattern': 'lib/**/*.dart'},
            result: '2 files',
            data: const {
              'paths': ['lib/main.dart', 'lib/app.dart'],
              'truncated': false,
            },
          ),
        ),
      ),
      size: const Size(340, 180),
    ),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          entry.value.widget,
          size: entry.value.size,
          brightness: brightness,
        );

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('../goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
