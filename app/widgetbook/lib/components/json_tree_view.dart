import 'package:assistant_ui/assistant_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

const _glob = <String, Object?>{
  'paths': ['lib/main.dart', 'lib/app.dart', 'test/app_test.dart'],
  'truncated': false,
};

const _nested = <String, Object?>{
  'pid': 4182,
  'status': 'running',
  'exit_code': null,
  'env': {
    'PATH': '/usr/bin',
    'limits': {'cpu': 2, 'memory_mb': 512},
  },
  'tags': ['build', 'ci'],
};

Widget _tree(Map<String, Object?> data, {int depth = 1}) => Padding(
  padding: const EdgeInsets.all(16),
  child: JsonTreeView(data: data, initiallyExpandedDepth: depth),
);

final jsonTreeViewComponent = WidgetbookComponent(
  name: 'JsonTreeView',
  useCases: [
    WidgetbookUseCase(name: 'collapsed', builder: (context) => _tree(_glob)),
    WidgetbookUseCase(
      name: 'nested, expanded',
      builder: (context) => _tree(_nested, depth: 3),
    ),
    WidgetbookUseCase(name: 'empty', builder: (context) => _tree(const {})),
  ],
);
