import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:assistant_widgetbook/main.dart';
import 'package:assistant_widgetbook/themes.dart';

void main() {
  testWidgets('the gallery builds with an empty registry', (tester) async {
    await tester.pumpWidget(const AssistantWidgetbook());
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.byType(AssistantWidgetbook), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('the theme addon themes material_ui widgets', (tester) async {
    late Brightness seen;
    final dark = kThemeAddon.themes.firstWhere((t) => t.name == 'Dark');

    await tester.pumpWidget(
      Builder(
        builder: (outer) => materialUiAppBuilder(
          outer,
          Builder(
            builder: (context) => kThemeAddon.themeBuilder(
              context,
              dark.data,
              Builder(
                builder: (inner) {
                  seen = Theme.of(inner).brightness;
                  return const SizedBox();
                },
              ),
            ),
          ),
        ),
      ),
    );

    expect(seen, Brightness.dark);
  });
}
