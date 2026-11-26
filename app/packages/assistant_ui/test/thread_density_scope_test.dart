import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

class _Probe extends StatelessWidget {
  const _Probe({required this.onBuild});

  final void Function(TimelineDensity density) onBuild;

  @override
  Widget build(BuildContext context) {
    final density = ThreadDensityScope.of(context);
    onBuild(density);
    return Text(density.name, textDirection: TextDirection.ltr);
  }
}

void main() {
  testWidgets('a descendant resolves the scope density', (tester) async {
    await tester.pumpWidget(
      ThreadDensityScope(
        density: TimelineDensity.compact,
        child: _Probe(onBuild: (_) {}),
      ),
    );

    expect(find.text('compact'), findsOneWidget);
  });

  testWidgets('rebuilding with a new density rebuilds dependents', (
    tester,
  ) async {
    final seen = <TimelineDensity>[];
    Widget scope(TimelineDensity density) => ThreadDensityScope(
      density: density,
      child: _Probe(onBuild: seen.add),
    );

    await tester.pumpWidget(scope(TimelineDensity.compact));
    await tester.pumpWidget(scope(TimelineDensity.expanded));

    expect(seen, [TimelineDensity.compact, TimelineDensity.expanded]);
    expect(find.text('expanded'), findsOneWidget);
  });

  testWidgets('an unchanged density does not rebuild dependents', (
    tester,
  ) async {
    var builds = 0;
    final probe = _Probe(onBuild: (_) => builds++);
    Widget scope() =>
        ThreadDensityScope(density: TimelineDensity.normal, child: probe);

    await tester.pumpWidget(scope());
    await tester.pumpWidget(scope());

    expect(builds, 1);
  });

  testWidgets('without a scope the density is normal', (tester) async {
    await tester.pumpWidget(_Probe(onBuild: (_) {}));

    expect(find.text('normal'), findsOneWidget);
  });

  testWidgets('the nearest scope wins', (tester) async {
    await tester.pumpWidget(
      ThreadDensityScope(
        density: TimelineDensity.expanded,
        child: ThreadDensityScope(
          density: TimelineDensity.compact,
          child: _Probe(onBuild: (_) {}),
        ),
      ),
    );

    expect(find.text('compact'), findsOneWidget);
  });

  testWidgets('fromWidth derives the density from the given width', (
    tester,
  ) async {
    await tester.pumpWidget(
      ThreadDensityScope.fromWidth(width: 320, child: _Probe(onBuild: (_) {})),
    );

    expect(find.text('compact'), findsOneWidget);
  });
}
