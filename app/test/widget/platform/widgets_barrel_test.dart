// Compile/import test for the façade barrel.
//
// This file imports ONLY the façade barrel and exercises a representative
// sample of widgets and types. If a `show`-list entry is accidentally
// dropped (or removed without a wrapper replacement), this file will fail
// to compile and the test will fail at load time — surfacing the show-list
// regression immediately.

import 'package:assistant_app/shared/platform/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('Façade barrel — show-list reachability', () {
    test('flutter/widgets.dart neutral primitives reachable', () {
      // These come from flutter/widgets.dart (always re-exported).
      // ignore: unnecessary_statements
      const SizedBox(width: 1, height: 1);
      // ignore: unnecessary_statements
      const Padding(padding: EdgeInsets.zero, child: SizedBox.shrink());
      // ignore: unnecessary_statements
      const Text('hi');
      // ignore: unnecessary_statements
      const Icon(IconData(0x100));
    });

    test('curated material widgets reachable via barrel', () {
      // Surface widgets without iOS equivalents (Card, InkWell, etc.)
      // ignore: unnecessary_statements
      const Card(child: SizedBox.shrink());
      // ignore: unnecessary_statements
      const Material(child: SizedBox.shrink());
      // ignore: unnecessary_statements
      const Divider();
      // ignore: unused_local_variable
      final ink = InkWell(onTap: () {}, child: const SizedBox.shrink());
      expect(ink, isNotNull);
    });

    test('icon glyph constants reachable from both worlds', () {
      // Material icons — high-volume use across features (299 sites).
      expect(Icons.refresh, isNotNull);
      expect(Icons.error_outline, isNotNull);
      expect(Icons.timeline, isNotNull);
      // Cupertino icons — needed for AdaptiveIcon's cupertino: arg without
      // importing cupertino.dart directly in feature code.
      expect(CupertinoIcons.refresh, isNotNull);
      expect(CupertinoIcons.back, isNotNull);
      expect(CupertinoIcons.add, isNotNull);
    });

    test('theme types reachable for Theme.of() reads', () {
      // Theme types are needed for type annotations on `Theme.of(context).colorScheme`.
      const ColorScheme cs = ColorScheme.light();
      expect(cs.error, isNotNull);
      // ignore: unnecessary_statements
      ThemeData;
      // ignore: unnecessary_statements
      TextTheme;
      // ignore: unnecessary_statements
      Brightness.dark;
    });

    test('progress indicator reachable', () {
      // ignore: unnecessary_statements
      const CircularProgressIndicator.adaptive();
    });

    test('Material-only widgets reachable (no iOS equivalent)', () {
      // FAB has no Cupertino convention; re-exported as-is.
      // ignore: unused_local_variable
      final fab = FloatingActionButton(
        onPressed: () {},
        child: const SizedBox.shrink(),
      );
      expect(fab, isNotNull);
      // Tooltip — iOS has no native tooltip; re-exported.
      // ignore: unnecessary_statements
      const Tooltip(message: 'hi', child: SizedBox.shrink());
    });

    test('adaptive wrappers reachable through the barrel', () {
      // These are the Phase 1 wrappers + the pre-existing AdaptiveScaffold/
      // AdaptiveNavBar. Re-exported alongside the curated material subset so
      // feature code only needs the one barrel import.
      // ignore: unused_local_variable
      const navBar = AdaptiveNavBar(title: 'x');
      // ignore: unused_local_variable
      const sliverNav = AdaptiveSliverNavBar(title: 'x');
      // ignore: unused_local_variable
      const listTile = AdaptiveListTile(title: Text('x'));
      // ignore: unused_local_variable
      final btn = AdaptiveButton.filled(
        onPressed: () {},
        child: const Text('x'),
      );
      expect(btn, isNotNull);
    });

    test('platform-style helpers reachable', () {
      // platformStyle is the single source of truth for adaptive branching.
      expect(platformStyle, isA<AppPlatformStyle>());
    });
  });
}
