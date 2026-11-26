import 'package:assistant_ui/assistant_ui.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:assistant_app/features/chat/chat_markdown_config.dart';

Future<ChatMarkdownScope> _pumpScope(WidgetTester tester) async {
  late ChatMarkdownScope scope;
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: ChatMarkdownConfig(
          child: Builder(
            builder: (context) {
              scope = ChatMarkdownScope.of(context)!;
              return const SizedBox.shrink();
            },
          ),
        ),
      ),
    ),
  );
  return scope;
}

void main() {
  group('ChatMarkdownConfig', () {
    testWidgets('installs the Mermaid and SVG plugins and builders', (
      tester,
    ) async {
      final scope = await _pumpScope(tester);

      expect(scope.plugins, isNotNull);
      expect(scope.builders, isNotNull);
      expect(scope.imageBuilder, same(networkMarkdownImage));
    });

    testWidgets('keeps the registries across rebuilds', (tester) async {
      final first = await _pumpScope(tester);
      final second = await _pumpScope(tester);

      expect(second.builders, same(first.builders));
      expect(second.plugins, same(first.plugins));
    });

    testWidgets('routes link taps through MarkdownLinkHandler', (tester) async {
      final scope = await _pumpScope(tester);

      scope.onTapLink!('ftp://example.com/file');
      await tester.pump();

      expect(find.text('Cannot open link: ftp://example.com/file'), findsOne);
    });
  });

  group('networkMarkdownImage', () {
    test('renders raster URLs with CachedNetworkImage', () {
      expect(
        networkMarkdownImage('https://example.com/a.png', 'a', null),
        isA<CachedNetworkImage>(),
      );
    });

    test('renders .svg URLs with SvgPicture', () {
      expect(
        networkMarkdownImage('https://example.com/a.svg', 'a', null),
        isA<SvgPicture>(),
      );
    });
  });
}
