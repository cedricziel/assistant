import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/error_screen.dart';

void main() {
  group('ErrorScreen', () {
    final details = FlutterErrorDetails(
      exception: UnsupportedError('Platform._operatingSystem'),
      library: 'dart:io',
      context: ErrorDescription('during build'),
      stack: StackTrace.current,
    );

    // -- Material (default platform) ------------------------------------------

    testWidgets('Material: renders error icon and heading', (tester) async {
      await tester.pumpWidget(MaterialApp(home: ErrorScreen(details: details)));

      expect(find.byIcon(Icons.error_outline), findsOneWidget);
      expect(find.text('Something went wrong'), findsOneWidget);
    });

    testWidgets('Material: shows exception text in debug mode', (tester) async {
      await tester.pumpWidget(MaterialApp(home: ErrorScreen(details: details)));

      expect(find.textContaining('Platform._operatingSystem'), findsWidgets);
    });

    testWidgets('Material: shows library and context info', (tester) async {
      await tester.pumpWidget(MaterialApp(home: ErrorScreen(details: details)));

      expect(find.textContaining('dart:io'), findsOneWidget);
      expect(find.textContaining('during build'), findsOneWidget);
    });

    testWidgets('Material: has ExpansionTile stack trace', (tester) async {
      await tester.pumpWidget(MaterialApp(home: ErrorScreen(details: details)));

      expect(find.text('Stack trace'), findsOneWidget);
      expect(find.byType(ExpansionTile), findsOneWidget);
    });

    testWidgets('Material: is scrollable', (tester) async {
      await tester.pumpWidget(MaterialApp(home: ErrorScreen(details: details)));

      expect(find.byType(SingleChildScrollView), findsOneWidget);
    });

    testWidgets('renders without stack trace when absent', (tester) async {
      final noStack = FlutterErrorDetails(exception: StateError('test error'));

      await tester.pumpWidget(MaterialApp(home: ErrorScreen(details: noStack)));

      expect(find.text('Something went wrong'), findsOneWidget);
      expect(find.text('Stack trace'), findsNothing);
    });

    // -- Cupertino (iOS) ------------------------------------------------------

    testWidgets('iOS: uses CupertinoNavigationBar', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        CupertinoApp(
          home: Theme(
            data: ThemeData(
              colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue),
            ),
            child: Material(
              type: MaterialType.transparency,
              child: ErrorScreen(details: details),
            ),
          ),
        ),
      );

      expect(find.byType(CupertinoNavigationBar), findsOneWidget);
      // The heading text is in the nav bar, not a Material icon row.
      expect(find.byIcon(Icons.error_outline), findsNothing);
      expect(find.text('Something went wrong'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('iOS: uses chevron toggle instead of ExpansionTile', (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        CupertinoApp(
          home: Theme(
            data: ThemeData(
              colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue),
            ),
            child: Material(
              type: MaterialType.transparency,
              child: ErrorScreen(details: details),
            ),
          ),
        ),
      );

      // Should use a CupertinoButton with chevron, not an ExpansionTile.
      expect(find.byType(ExpansionTile), findsNothing);
      expect(find.byType(CupertinoButton), findsOneWidget);
      expect(find.text('Stack trace'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    // -- ErrorWidget.builder wiring -------------------------------------------

    testWidgets('ErrorWidget.builder wiring shows ErrorScreen', (tester) async {
      final originalBuilder = ErrorWidget.builder;
      ErrorWidget.builder = (details) => ErrorScreen(details: details);

      final errors = <FlutterErrorDetails>[];
      final originalOnError = FlutterError.onError;
      FlutterError.onError = (d) => errors.add(d);

      try {
        await tester.pumpWidget(
          MaterialApp(
            home: Builder(
              builder: (context) {
                throw UnsupportedError('Platform._operatingSystem');
              },
            ),
          ),
        );

        expect(find.text('Something went wrong'), findsOneWidget);
        expect(find.textContaining('Platform._operatingSystem'), findsWidgets);

        expect(errors, hasLength(1));
        expect(
          errors.first.exceptionAsString(),
          contains('Platform._operatingSystem'),
        );
      } finally {
        ErrorWidget.builder = originalBuilder;
        FlutterError.onError = originalOnError;
      }
    });
  });
}
