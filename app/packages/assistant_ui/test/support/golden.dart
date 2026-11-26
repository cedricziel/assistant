import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

/// Fixed wall clock for elements that show relative or elapsed time.
final kGoldenNow = DateTime.utc(2026, 1, 15, 12);

/// Brand seed and accent roles of the app's theme (`assistant_theme.dart`).
/// Duplicated here because the package cannot import the app.
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

/// Carries the injected clock to the widget under test.
///
/// Elements take `now` by parameter; tests read it here and pass it in.
class GoldenClock extends InheritedWidget {
  const GoldenClock({super.key, required this.now, required super.child});

  final DateTime now;

  static DateTime of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<GoldenClock>()!.now;

  @override
  bool updateShouldNotify(GoldenClock oldWidget) => now != oldWidget.now;
}

/// Pumps [widget] in a [MaterialApp] themed like the app, at a fixed logical
/// [size] and device pixel ratio of 1.
///
/// Advance time with `tester.pump(Duration)`. `pumpAndSettle` is banned in
/// golden tests: repeating animations such as the typing dots never settle.
Future<void> pumpGolden(
  WidgetTester tester,
  Widget widget, {
  Size size = const Size(400, 300),
  Brightness brightness = Brightness.light,
  DateTime? now,
  bool disableAnimations = false,
}) async {
  tester.view
    ..physicalSize = size
    ..devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    GoldenClock(
      now: now ?? kGoldenNow,
      child: MaterialApp(
        debugShowCheckedModeBanner: false,
        theme: goldenTheme(Brightness.light),
        darkTheme: goldenTheme(Brightness.dark),
        themeMode: brightness == Brightness.dark
            ? ThemeMode.dark
            : ThemeMode.light,
        builder: (context, child) => MediaQuery(
          data: MediaQuery.of(context)
              .copyWith(disableAnimations: disableAnimations),
          child: child!,
        ),
        home: Scaffold(body: Center(child: widget)),
      ),
    ),
  );
}
