// RED→GREEN: openspec/changes/generative-ui-a2ui PR 14b.3 and 14b.4.
import 'package:assistant_app/features/chat/a2ui/a2ui_surface_view.dart';
import 'package:assistant_app/features/chat/a2ui/assistant_catalog.dart';
import 'package:assistant_app/features/chat/a2ui/link_opener.dart';
import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:url_launcher/url_launcher.dart';

import 'a2ui_test_support.dart';

List<Map<String, Object?>> _linkButton(String url) => surfaceMessages([
  {
    'id': 'root',
    'component': 'Button',
    'child': 'label',
    'action': {
      'functionCall': {
        '@call': 'openUrl',
        'args': {'url': url},
      },
    },
  },
  {'id': 'label', 'component': 'Text', 'text': 'Visit'},
]);

class _Launches {
  final List<Uri> uris = [];
  final List<LaunchMode> modes = [];

  Future<bool> call(
    Uri uri, {
    LaunchMode mode = LaunchMode.platformDefault,
  }) async {
    uris.add(uri);
    modes.add(mode);
    return true;
  }
}

Widget _app(String url, _Launches launches, {List<String> hosts = const []}) {
  final key = GlobalKey();
  final opener = confirmingLinkOpener(
    context: () => key.currentContext!,
    linkHosts: () => hosts,
    launcher: launches.call,
  );
  return host(
    Builder(
      key: key,
      builder: (context) => A2uiSurfaceView(
        messages: _linkButton(url),
        catalog: assistantCatalog(
          imageLoader: FakeAttachmentImageLoader(),
          openUrl: opener,
        ),
      ),
    ),
  );
}

Future<void> _tapVisit(WidgetTester tester) async {
  await tester.pump();
  await tester.tap(find.text('Visit'));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('a host outside the list asks, showing the full URL', (
    tester,
  ) async {
    final launches = _Launches();
    await tester.pumpWidget(_app('https://example.org/a?b=1', launches));
    await _tapVisit(tester);

    expect(find.byType(AlertDialog), findsOneWidget);
    expect(
      find.textContaining('https://example.org/a?b=1', findRichText: true),
      findsOneWidget,
    );
    expect(find.text('Open'), findsOneWidget);
    expect(find.text('Cancel'), findsOneWidget);
    expect(launches.uris, isEmpty, reason: 'nothing launches before Open');
  });

  testWidgets('the host is emphasised in the destination', (tester) async {
    final launches = _Launches();
    await tester.pumpWidget(_app('https://example.org/a', launches));
    await _tapVisit(tester);

    final rich = tester
        .widgetList<RichText>(find.byType(RichText))
        .where((r) => r.text.toPlainText() == 'https://example.org/a');
    expect(rich, hasLength(1));
    final spans = <TextSpan>[];
    rich.single.text.visitChildren((s) {
      if (s is TextSpan && s.text != null) spans.add(s);
      return true;
    });
    final bold = spans.where((s) => s.style?.fontWeight == FontWeight.bold);
    expect(bold.map((s) => s.text), ['example.org']);
  });

  testWidgets('the host after user info is emphasised, not the user name', (
    tester,
  ) async {
    final launches = _Launches();
    await tester.pumpWidget(_app('https://evil.test@evil.test/a', launches));
    await _tapVisit(tester);

    final rich = tester
        .widgetList<RichText>(find.byType(RichText))
        .where((r) => r.text.toPlainText() == 'https://evil.test@evil.test/a')
        .single;
    final parts = <String>[];
    rich.text.visitChildren((s) {
      if (s is TextSpan && s.style?.fontWeight == FontWeight.bold) {
        parts.add(s.text!);
      }
      return true;
    });
    expect(parts, ['evil.test']);
    final spans = <String>[];
    rich.text.visitChildren((s) {
      if (s is TextSpan && s.text != null) spans.add(s.text!);
      return true;
    });
    expect(spans.first, 'https://evil.test@', reason: 'bold is the real host');
  });

  testWidgets('a very long URL scrolls inside the dialog', (tester) async {
    final launches = _Launches();
    await tester.pumpWidget(
      _app('https://example.org/${'a' * 3000}', launches),
    );
    await _tapVisit(tester);

    expect(find.byType(SingleChildScrollView), findsWidgets);
    expect(tester.takeException(), isNull);
    expect(find.text('Open'), findsOneWidget);
  });

  testWidgets('Cancel launches nothing', (tester) async {
    final launches = _Launches();
    await tester.pumpWidget(_app('https://example.org/a', launches));
    await _tapVisit(tester);

    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();

    expect(find.byType(AlertDialog), findsNothing);
    expect(launches.uris, isEmpty);
  });

  testWidgets('Open launches once, externally', (tester) async {
    final launches = _Launches();
    await tester.pumpWidget(_app('https://example.org/a', launches));
    await _tapVisit(tester);

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();

    expect(launches.uris, [Uri.parse('https://example.org/a')]);
    expect(launches.modes, [LaunchMode.externalApplication]);
  });

  testWidgets('a listed host opens without a dialog, case-insensitively', (
    tester,
  ) async {
    final launches = _Launches();
    await tester.pumpWidget(
      _app('https://DOCS.example/x', launches, hosts: ['docs.example']),
    );
    await _tapVisit(tester);

    expect(find.byType(AlertDialog), findsNothing);
    expect(launches.uris, [Uri.parse('https://DOCS.example/x')]);
  });

  testWidgets('a host that only contains a listed host still asks', (
    tester,
  ) async {
    final launches = _Launches();
    await tester.pumpWidget(
      _app(
        'https://docs.example.evil.test/x',
        launches,
        hosts: ['docs.example'],
      ),
    );
    await _tapVisit(tester);

    expect(find.byType(AlertDialog), findsOneWidget);
    expect(launches.uris, isEmpty);
  });

  testWidgets('mailto always asks and shows the address', (tester) async {
    final launches = _Launches();
    await tester.pumpWidget(
      _app('mailto:a@example.org', launches, hosts: ['example.org']),
    );
    await _tapVisit(tester);

    expect(find.byType(AlertDialog), findsOneWidget);
    expect(
      find.textContaining('a@example.org', findRichText: true),
      findsOneWidget,
    );
    expect(launches.uris, isEmpty);
  });

  for (final url in [
    'javascript:alert(1)',
    'data:text/html;base64,PHNjcmlwdD4=',
    'http://example.org/a',
    'tel:+4912345',
  ]) {
    testWidgets('$url never reaches the dialog or the launcher', (
      tester,
    ) async {
      final launches = _Launches();
      await tester.pumpWidget(_app(url, launches, hosts: ['example.org']));
      await _tapVisit(tester);

      expect(find.byType(AlertDialog), findsNothing);
      expect(find.byType(CupertinoAlertDialog), findsNothing);
      expect(launches.uris, isEmpty);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('the dialog is Cupertino on iOS', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    final launches = _Launches();
    await tester.pumpWidget(_app('https://example.org/a', launches));
    await _tapVisit(tester);

    expect(find.byType(CupertinoAlertDialog), findsOneWidget);
    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    expect(launches.uris, hasLength(1));
  });
}
