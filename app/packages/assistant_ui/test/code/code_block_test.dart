import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

Widget _host(
  Widget child, {
  Brightness brightness = Brightness.light,
  bool disableAnimations = false,
}) => MaterialApp(
  theme: goldenTheme(brightness),
  builder: (context, app) => MediaQuery(
    data: MediaQuery.of(context).copyWith(disableAnimations: disableAnimations),
    child: app!,
  ),
  home: Scaffold(
    body: Align(alignment: Alignment.topLeft, child: child),
  ),
);

Finder get _copyButton => find.bySemanticsLabel('Copy code');

Finder get _body => find.byKey(const Key('code_block_body'));

TextSpan _bodySpan(WidgetTester tester) =>
    tester
            .widget<Text>(
              find.descendant(of: _body, matching: find.byType(Text)),
            )
            .textSpan!
        as TextSpan;

Iterable<TextSpan> _leaves(TextSpan span) sync* {
  if (span.children == null || span.children!.isEmpty) {
    yield span;
    return;
  }
  for (final child in span.children!) {
    yield* _leaves(child as TextSpan);
  }
}

void main() {
  group('language label', () {
    testWidgets('shows the fence language as written', (tester) async {
      await tester.pumpWidget(
        _host(const CodeBlock(code: 'fn main() {}', language: 'rust')),
      );

      expect(find.text('rust'), findsOneWidget);
    });

    testWidgets('shows text when the language is absent', (tester) async {
      await tester.pumpWidget(_host(const CodeBlock(code: 'hello')));

      expect(find.text('text'), findsOneWidget);
    });
  });

  testWidgets('the header does not overlap the code', (tester) async {
    await tester.pumpWidget(
      _host(
        SizedBox(
          width: 360,
          child: CodeBlock(code: 'x' * 300, language: 'rust', onCopy: (_) {}),
        ),
      ),
    );

    final button = tester.getRect(_copyButton);
    final code = tester.getRect(
      find.descendant(of: _body, matching: find.byType(RichText)),
    );

    expect(button.overlaps(code), isFalse);
  });

  testWidgets('select all and copy excludes the label', (tester) async {
    String? clipboard;
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'Clipboard.setData') {
          clipboard = (call.arguments as Map)['text'] as String?;
        }
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );

    await tester.pumpWidget(
      _host(
        SelectionArea(
          child: CodeBlock(
            code: 'fn main() {}',
            language: 'rust',
            onCopy: (_) {},
          ),
        ),
      ),
    );
    await tester.tapAt(tester.getCenter(_body));
    await tester.pump();
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyA);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyC);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.pump();

    expect(clipboard, 'fn main() {}');
    expect(clipboard, isNot(contains('rust')));
    expect(clipboard, isNot(contains('Copy')));
  });

  testWidgets('a 300-character line scrolls at 360 px without overflow', (
    tester,
  ) async {
    tester.view
      ..physicalSize = const Size(360, 400)
      ..devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      _host(CodeBlock(code: 'a' * 300, language: 'dart', onCopy: (_) {})),
    );

    expect(tester.takeException(), isNull);
    final scroll = tester.widget<SingleChildScrollView>(_body);
    expect(scroll.scrollDirection, Axis.horizontal);
    final position = tester
        .state<ScrollableState>(
          find.descendant(of: _body, matching: find.byType(Scrollable)),
        )
        .position;
    expect(position.maxScrollExtent, greaterThan(0));
  });

  group('copy', () {
    testWidgets('onCopy receives the exact code, then Copied, then reverts', (
      tester,
    ) async {
      final copied = <String>[];
      await tester.pumpWidget(
        _host(
          CodeBlock(
            code: 'fn main() {}\n',
            language: 'rust',
            onCopy: copied.add,
          ),
        ),
      );

      await tester.tap(_copyButton);
      await tester.pump();

      expect(copied, ['fn main() {}\n']);
      expect(find.bySemanticsLabel('Copied'), findsOneWidget);
      expect(_copyButton, findsNothing);
      expect(find.byIcon(Icons.check), findsOneWidget);

      await tester.pump(const Duration(seconds: 2));

      expect(_copyButton, findsOneWidget);
      expect(find.bySemanticsLabel('Copied'), findsNothing);
    });

    testWidgets('no callback means no button', (tester) async {
      await tester.pumpWidget(
        _host(const CodeBlock(code: 'x', language: 'rust')),
      );

      expect(_copyButton, findsNothing);
      expect(find.byType(InkWell), findsNothing);
    });

    testWidgets('Tab then Enter activates copy', (tester) async {
      final copied = <String>[];
      await tester.pumpWidget(
        _host(
          CodeBlock(code: 'let x = 1;', language: 'rust', onCopy: copied.add),
        ),
      );

      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pump();

      expect(copied, ['let x = 1;']);
    });

    testWidgets('the copy target is at least 24x24', (tester) async {
      await tester.pumpWidget(
        _host(CodeBlock(code: 'x', language: 'rust', onCopy: (_) {})),
      );

      final size = tester.getSize(find.byType(InkWell));

      expect(size.width, greaterThanOrEqualTo(24));
      expect(size.height, greaterThanOrEqualTo(24));
    });

    testWidgets('Copied appears without a fade under disableAnimations', (
      tester,
    ) async {
      await tester.pumpWidget(
        _host(
          CodeBlock(code: 'x', language: 'rust', onCopy: (_) {}),
          disableAnimations: true,
        ),
      );

      await tester.tap(_copyButton);
      await tester.pump();

      expect(find.bySemanticsLabel('Copied'), findsOneWidget);
      expect(
        find.descendant(
          of: find.byType(CodeBlock),
          matching: find.byType(AnimatedSwitcher),
        ),
        findsNothing,
      );
      expect(
        find.descendant(
          of: find.byType(CodeBlock),
          matching: find.byType(FadeTransition),
        ),
        findsNothing,
      );
      expect(
        find.descendant(
          of: find.byType(CodeBlock),
          matching: find.byType(AnimatedOpacity),
        ),
        findsNothing,
      );
    });
  });

  group('themes', () {
    testWidgets('dark paints CodeTheme.dark and its token colours', (
      tester,
    ) async {
      await tester.pumpWidget(
        _host(
          const CodeBlock(code: 'let x = 1;', language: 'rust'),
          brightness: Brightness.dark,
        ),
      );

      final decorated = tester.widget<DecoratedBox>(
        find.ancestor(of: _body, matching: find.byType(DecoratedBox)).first,
      );
      expect(
        (decorated.decoration as BoxDecoration).color,
        CodeTheme.dark.background,
      );
      final keyword = _leaves(_bodySpan(tester))
          .firstWhere((s) => s.text == 'let');
      expect(keyword.style?.color, CodeTheme.dark.colorFor('keyword'));
    });

    testWidgets('light paints CodeTheme.light', (tester) async {
      await tester.pumpWidget(
        _host(const CodeBlock(code: 'let x = 1;', language: 'rust')),
      );

      final decorated = tester.widget<DecoratedBox>(
        find.ancestor(of: _body, matching: find.byType(DecoratedBox)).first,
      );
      expect(
        (decorated.decoration as BoxDecoration).color,
        CodeTheme.light.background,
      );
    });
  });

  group('highlighting', () {
    testWidgets('an unknown language renders one plain span', (tester) async {
      await tester.pumpWidget(
        _host(const CodeBlock(code: 'let x = 1;', language: 'not-a-language')),
      );

      final leaves = _leaves(_bodySpan(tester)).toList();
      expect(leaves, hasLength(1));
      expect(leaves.single.style?.color, isNull);
      expect(leaves.single.text, 'let x = 1;');
    });

    testWidgets('highlight: false renders plain', (tester) async {
      await tester.pumpWidget(
        _host(
          const CodeBlock(
            code: 'let x = 1;',
            language: 'rust',
            highlight: false,
          ),
        ),
      );

      expect(_leaves(_bodySpan(tester)), hasLength(1));
    });
  });
}
