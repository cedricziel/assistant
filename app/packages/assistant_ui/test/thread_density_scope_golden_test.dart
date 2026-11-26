@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/golden.dart';

/// Stands in for a density-dependent element: it prints the density it
/// resolves from the nearest [ThreadDensityScope].
class _DensityProbe extends StatelessWidget {
  const _DensityProbe();

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.all(16),
    child: Text('density: ${ThreadDensityScope.of(context).name}'),
  );
}

void main() {
  final cases = <String, Widget>{
    for (final density in TimelineDensity.values)
      'thread_density_scope_${density.name}': ThreadDensityScope(
        density: density,
        child: const _DensityProbe(),
      ),
    'thread_density_scope_none': const _DensityProbe(),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          entry.value,
          size: const Size(240, 64),
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 50));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
