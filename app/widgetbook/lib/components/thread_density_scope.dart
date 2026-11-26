import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

import '../knobs.dart';

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

final threadDensityScopeComponent = WidgetbookComponent(
  name: 'ThreadDensityScope',
  useCases: [
    WidgetbookUseCase(
      name: 'density',
      builder: (context) => ThreadDensityScope(
        density: enumKnob(
          context,
          label: 'density',
          values: TimelineDensity.values,
          initial: TimelineDensity.normal,
        ),
        child: const _DensityProbe(),
      ),
    ),
    WidgetbookUseCase(
      name: 'no scope (normal)',
      builder: (context) => const _DensityProbe(),
    ),
  ],
);
