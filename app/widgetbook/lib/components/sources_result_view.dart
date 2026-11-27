import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

const _valid = <String, Object?>{
  'kind': 'sources',
  'query': 'rust async traits',
  'results': [
    {
      'title': 'Async functions in traits',
      'url':
          'https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html',
      'domain': 'blog.rust-lang.org',
      'snippet': 'Async fn in traits is stable in Rust 1.75.',
    },
    {
      'title': 'The async-trait crate',
      'url': 'https://docs.rs/async-trait',
      'domain': 'docs.rs',
    },
  ],
};

const _empty = <String, Object?>{
  'kind': 'sources',
  'query': 'no hits for this',
  'results': <Object?>[],
};

/// Fails the contract: every result needs a `url`.
const _invalid = <String, Object?>{
  'kind': 'sources',
  'query': 'q',
  'results': [
    {'title': 'No link', 'domain': 'example.com'},
  ],
};

Widget _result(
  Map<String, Object?> data, {
  ToolCallStatus status = ToolCallStatus.ok,
  TimelineDensity density = TimelineDensity.normal,
}) => Padding(
  padding: const EdgeInsets.all(16),
  child: ThreadDensityScope(
    density: density,
    child: Builder(
      builder: (context) {
        final record = ToolCallRecord(
          toolName: 'web-search',
          status: status,
          data: data,
        );
        return ToolResultRendererRegistry(byKind: defaultToolResultRenderers())
                .build(context, record) ??
            GenericToolResultView(record: record);
      },
    ),
  ),
);

final sourcesResultViewComponent = WidgetbookComponent(
  name: 'SourcesResultView',
  useCases: [
    WidgetbookUseCase(name: 'valid', builder: (context) => _result(_valid)),
    WidgetbookUseCase(name: 'empty', builder: (context) => _result(_empty)),
    WidgetbookUseCase(
      name: 'invalid falls back',
      builder: (context) => _result(_invalid),
    ),
    WidgetbookUseCase(
      name: 'error status',
      builder: (context) => _result(_valid, status: ToolCallStatus.error),
    ),
    for (final density in TimelineDensity.values)
      WidgetbookUseCase(
        name: 'density ${density.name}',
        builder: (context) => _result(_valid, density: density),
      ),
  ],
);
