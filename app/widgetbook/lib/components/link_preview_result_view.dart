import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

const _full = <String, Object?>{
  'kind': 'link_preview',
  'url': 'https://example.com/a',
  'final_url': 'https://www.example.com/a/',
  'status': 200,
  'title': 'Example',
  'content_type': 'text/html; charset=utf-8',
  'excerpt': 'A short excerpt of the page.',
};

const _missing = <String, Object?>{
  'kind': 'link_preview',
  'url': 'https://example.com/missing',
  'final_url': 'https://example.com/missing',
  'status': 404,
};

/// Fails the contract: `status` must be an integer.
const _invalid = <String, Object?>{
  'kind': 'link_preview',
  'url': 'https://example.com',
  'final_url': 'https://example.com',
  'status': 'ok',
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
          toolName: 'web-fetch',
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

final linkPreviewResultViewComponent = WidgetbookComponent(
  name: 'LinkPreviewResultView',
  useCases: [
    WidgetbookUseCase(name: 'full', builder: (context) => _result(_full)),
    WidgetbookUseCase(name: '404', builder: (context) => _result(_missing)),
    WidgetbookUseCase(
      name: 'invalid falls back',
      builder: (context) => _result(_invalid),
    ),
    WidgetbookUseCase(
      name: 'error status',
      builder: (context) => _result(_full, status: ToolCallStatus.error),
    ),
    for (final density in TimelineDensity.values)
      WidgetbookUseCase(
        name: 'density ${density.name}',
        builder: (context) => _result(_full, density: density),
      ),
  ],
);
