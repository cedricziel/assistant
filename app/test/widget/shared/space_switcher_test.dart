import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/spaces/space_provider.dart';
import 'package:assistant_app/shared/space_switcher.dart';

GoRouter _router({String initial = '/home'}) => GoRouter(
  initialLocation: initial,
  routes: [
    GoRoute(
      path: '/home',
      builder: (ctx, s) => const Scaffold(body: _TestSpaceSwitcherExpanded()),
    ),
    GoRoute(
      path: '/home-collapsed',
      builder: (ctx, s) => const Scaffold(body: _TestSpaceSwitcherCollapsed()),
    ),
    GoRoute(
      path: '/spaces',
      builder: (ctx, s) => const Scaffold(body: Text('Space Selector')),
    ),
  ],
);

/// Wraps the expanded space switcher for testing.
class _TestSpaceSwitcherExpanded extends ConsumerWidget {
  const _TestSpaceSwitcherExpanded();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return const SpaceSwitcher(collapsed: false);
  }
}

/// Wraps the collapsed space switcher for testing.
class _TestSpaceSwitcherCollapsed extends ConsumerWidget {
  const _TestSpaceSwitcherCollapsed();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return const SpaceSwitcher(collapsed: true);
  }
}

void main() {
  group('SpaceSwitcher', () {
    testWidgets('shows org and space name when selection is complete', (
      tester,
    ) async {
      final container = ProviderContainer(
        overrides: [
          spaceSelectionProvider.overrideWith(
            () => _PreloadedSelectionNotifier(
              const SpaceSelection(
                orgId: 'org-1',
                orgName: 'Acme Corp',
                spaceId: 'space-1',
                spaceName: 'Engineering',
              ),
            ),
          ),
        ],
      );
      addTearDown(container.dispose);

      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp.router(routerConfig: _router()),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Acme Corp'), findsOneWidget);
      expect(find.text('Engineering'), findsOneWidget);
    });

    testWidgets('shows placeholder when no selection', (tester) async {
      await tester.pumpWidget(
        ProviderScope(child: MaterialApp.router(routerConfig: _router())),
      );
      await tester.pumpAndSettle();

      expect(find.text('Select space'), findsOneWidget);
    });

    testWidgets('shows org name only when space not yet selected', (
      tester,
    ) async {
      final container = ProviderContainer(
        overrides: [
          spaceSelectionProvider.overrideWith(
            () => _PreloadedSelectionNotifier(
              const SpaceSelection(orgId: 'org-1', orgName: 'Acme Corp'),
            ),
          ),
        ],
      );
      addTearDown(container.dispose);

      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp.router(routerConfig: _router()),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Acme Corp'), findsOneWidget);
      expect(find.text('Select space'), findsOneWidget);
    });

    testWidgets('navigates to /spaces on tap', (tester) async {
      final container = ProviderContainer(
        overrides: [
          spaceSelectionProvider.overrideWith(
            () => _PreloadedSelectionNotifier(
              const SpaceSelection(
                orgId: 'org-1',
                orgName: 'Acme Corp',
                spaceId: 'space-1',
                spaceName: 'Engineering',
              ),
            ),
          ),
        ],
      );
      addTearDown(container.dispose);

      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp.router(routerConfig: _router()),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byType(SpaceSwitcher));
      await tester.pumpAndSettle();

      expect(find.text('Space Selector'), findsOneWidget);
    });

    testWidgets('shows tooltip with icon-only in collapsed mode', (
      tester,
    ) async {
      final container = ProviderContainer(
        overrides: [
          spaceSelectionProvider.overrideWith(
            () => _PreloadedSelectionNotifier(
              const SpaceSelection(
                orgId: 'org-1',
                orgName: 'Acme Corp',
                spaceId: 'space-1',
                spaceName: 'Engineering',
              ),
            ),
          ),
        ],
      );
      addTearDown(container.dispose);

      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp.router(
            routerConfig: _router(initial: '/home-collapsed'),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // In collapsed mode, org/space text is not shown.
      expect(find.text('Acme Corp'), findsNothing);
      expect(find.text('Engineering'), findsNothing);

      // A Tooltip should be present.
      expect(find.byType(Tooltip), findsOneWidget);
    });

    testWidgets('collapsed mode navigates to /spaces on tap', (tester) async {
      final container = ProviderContainer(
        overrides: [
          spaceSelectionProvider.overrideWith(
            () => _PreloadedSelectionNotifier(
              const SpaceSelection(
                orgId: 'org-1',
                orgName: 'Acme Corp',
                spaceId: 'space-1',
                spaceName: 'Engineering',
              ),
            ),
          ),
        ],
      );
      addTearDown(container.dispose);

      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp.router(
            routerConfig: _router(initial: '/home-collapsed'),
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byType(SpaceSwitcher));
      await tester.pumpAndSettle();

      expect(find.text('Space Selector'), findsOneWidget);
    });
  });
}

class _PreloadedSelectionNotifier extends SpaceSelectionNotifier {
  _PreloadedSelectionNotifier(this._initial);
  final SpaceSelection _initial;

  @override
  SpaceSelection build() => _initial;
}
