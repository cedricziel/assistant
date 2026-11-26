import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'golden.dart';

void main() {
  group('pumpGolden', () {
    testWidgets('applies the requested logical size', (tester) async {
      await pumpGolden(
        tester,
        const SizedBox.expand(key: Key('probe')),
        size: const Size(320, 200),
      );

      expect(
        tester.getSize(find.byKey(const Key('probe'))),
        const Size(320, 200),
      );
    });

    for (final brightness in Brightness.values) {
      testWidgets('builds a ${brightness.name} theme from a seed', (
        tester,
      ) async {
        late ThemeData theme;
        await pumpGolden(
          tester,
          Builder(
            builder: (context) {
              theme = Theme.of(context);
              return const SizedBox();
            },
          ),
          brightness: brightness,
        );

        expect(theme.brightness, brightness);
        expect(theme.useMaterial3, isTrue);
      });
    }

    testWidgets('exposes a fixed clock, overridable per call', (tester) async {
      late DateTime seen;
      Widget probe() => Builder(
        builder: (context) {
          seen = GoldenClock.of(context);
          return const SizedBox();
        },
      );

      await pumpGolden(tester, probe());
      expect(seen, kGoldenNow);

      final custom = DateTime.utc(2030, 5, 6, 7);
      await pumpGolden(tester, probe(), now: custom);
      expect(seen, custom);
    });

    testWidgets('can request reduced motion', (tester) async {
      late bool disabled;
      await pumpGolden(
        tester,
        Builder(
          builder: (context) {
            disabled = MediaQuery.disableAnimationsOf(context);
            return const SizedBox();
          },
        ),
        disableAnimations: true,
      );

      expect(disabled, isTrue);
    });
  });
}
