import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:assistant_app/shared/platform/adaptive_app.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

/// Fixed wall clock for goldens that show relative or elapsed time.
///
/// The current chat widgets read `DateTime.now()` themselves, so
/// characterization tests derive their timestamps from the real clock at a safe
/// offset (see [goldenAgo]); this constant is for elements that take `now` by
/// parameter once they are extracted.
final kGoldenNow = DateTime.utc(2026, 1, 15, 12);

/// Brand seed and accent roles of the app's theme (`assistant_theme.dart`),
/// duplicated from `packages/assistant_ui/test/support/golden.dart` so that
/// characterization images and element images are rendered identically.
const _seed = Color(0xFF1A73E8);
const _secondary = Color(0xFF6D5BBA);
const _tertiary = Color(0xFF1F8A5C);

ThemeData goldenTheme(Brightness brightness) {
  final scheme = ColorScheme.fromSeed(
    seedColor: _seed,
    brightness: brightness,
  ).copyWith(secondary: _secondary, tertiary: _tertiary);
  return ThemeData(
    useMaterial3: true,
    colorScheme: scheme,
    brightness: brightness,
  );
}

/// A time [d] before the real clock, for widgets that format `DateTime.now()`.
///
/// Pick offsets in the middle of a bucket (for example 2.5 hours for "2h ago")
/// so the rendered label does not change between building the fixture and the
/// first frame.
DateTime goldenAgo(Duration d) => DateTime.now().subtract(d);

/// Encodes a solid 8x8 PNG, for stubbing image bytes without any I/O.
///
/// Must run inside `tester.runAsync` because image encoding is real async work.
Future<Uint8List> solidPng(Color color) async {
  final recorder = ui.PictureRecorder();
  Canvas(recorder)
      .drawRect(const Rect.fromLTWH(0, 0, 8, 8), Paint()..color = color);
  final image = await recorder.endRecording().toImage(8, 8);
  final data = await image.toByteData(format: ui.ImageByteFormat.png);
  return data!.buffer.asUint8List();
}

/// Pumps [widget] in a [MaterialApp] themed like the app, at a fixed logical
/// [size] and device pixel ratio of 1, optionally inside a [ProviderScope].
///
/// Advance time with `tester.pump(Duration)`. `pumpAndSettle` is banned in
/// golden tests: spinners and the typing dots never settle.
Future<void> pumpGolden(
  WidgetTester tester,
  Widget widget, {
  Size size = const Size(400, 300),
  Brightness brightness = Brightness.light,
  bool disableAnimations = false,
  List<Override> overrides = const [],
  Alignment alignment = Alignment.center,
}) async {
  tester.view
    ..physicalSize = size
    ..devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    ProviderScope(
      overrides: overrides,
      child: MaterialApp(
        debugShowCheckedModeBanner: false,
        theme: goldenTheme(Brightness.light),
        darkTheme: goldenTheme(Brightness.dark),
        themeMode: brightness == Brightness.dark
            ? ThemeMode.dark
            : ThemeMode.light,
        // Same bridge the app installs (adaptive_app.dart); flutter_smooth_markdown
        // still reads the framework theme and localizations through it.
        builder: (context, child) => LegacyDesignBridge(
          child: MediaQuery(
            data: MediaQuery.of(context)
                .copyWith(disableAnimations: disableAnimations),
            child: child!,
          ),
        ),
        home: Scaffold(
          body: Align(alignment: alignment, child: widget),
        ),
      ),
    ),
  );
}

/// Compares the [MaterialApp] against `goldens/<name>.png`, next to the test.
Future<void> expectGolden(String name) => expectLater(
  find.byType(MaterialApp),
  matchesGoldenFile('goldens/$name.png'),
);
