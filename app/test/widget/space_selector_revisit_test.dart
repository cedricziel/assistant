// Asserts that revisiting the SpaceSelectorScreen with a single space and
// a prior selection does NOT leave the user staring at an indefinite spinner.
// The cold-cache one-space flow still auto-navigates to /chat.
//
// Spec: openspec/specs/space-selector-resilience/spec.md
//   "Single-space revisit does not show an infinite spinner"

import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/spaces/space_provider.dart';
import 'package:assistant_app/features/spaces/space_selector_screen.dart';

OrgSummary _org() => OrgSummary(
  (b) => b
    ..authMode = 'password'
    ..id = 'org-1'
    ..name = 'Default Organization'
    ..slug = 'default',
);

SpaceSummary _space(String id, String name, String slug) => SpaceSummary(
  (b) => b
    ..id = id
    ..name = name
    ..slug = slug,
);

class _StaticOrgsNotifier extends OrgsNotifier {
  _StaticOrgsNotifier(this._orgs);
  final List<OrgSummary> _orgs;
  @override
  Future<List<OrgSummary>> build() async => _orgs;
}

class _StaticSpacesNotifier extends SpacesNotifier {
  _StaticSpacesNotifier(this._spaces);
  final List<SpaceSummary> _spaces;
  @override
  Future<List<SpaceSummary>> build() async {
    // Mirror the real provider's dependency on selection so it rebuilds when
    // the user toggles selection.
    ref.watch(spaceSelectionProvider);
    return _spaces;
  }
}

Widget _harness({
  required ProviderContainer container,
  required GoRouter router,
}) {
  return UncontrolledProviderScope(
    container: container,
    child: MaterialApp.router(routerConfig: router),
  );
}

GoRouter _router() => GoRouter(
  initialLocation: '/spaces',
  routes: [
    GoRoute(path: '/spaces', builder: (_, _) => const SpaceSelectorScreen()),
    GoRoute(
      path: '/chat',
      builder: (_, _) => const Scaffold(body: Text('CHAT_PAGE')),
    ),
  ],
);

void main() {
  testWidgets('first-time auto-select on single-space org navigates to /chat', (
    tester,
  ) async {
    final space = _space('sp-1', 'Default Space', 'default');
    final container = ProviderContainer(
      overrides: [
        orgsProvider.overrideWith(() => _StaticOrgsNotifier([_org()])),
        spacesProvider.overrideWith(() => _StaticSpacesNotifier([space])),
      ],
    );
    addTearDown(container.dispose);

    await tester.pumpWidget(_harness(container: container, router: _router()));
    await tester.pumpAndSettle();

    // Selection has been written by the auto-select callbacks.
    final selection = container.read(spaceSelectionProvider);
    expect(selection.orgId, 'org-1');
    expect(selection.spaceId, 'sp-1');
    // And the router has navigated to /chat.
    expect(find.text('CHAT_PAGE'), findsOneWidget);
  });

  testWidgets(
    'revisit with single space + existing selection renders the list, not a stuck spinner',
    (tester) async {
      final space = _space('sp-1', 'Default Space', 'default');
      final container = ProviderContainer(
        overrides: [
          orgsProvider.overrideWith(() => _StaticOrgsNotifier([_org()])),
          spacesProvider.overrideWith(() => _StaticSpacesNotifier([space])),
        ],
      );
      addTearDown(container.dispose);

      // Pre-seed the selection — same as if the user had already auto-selected
      // and is now coming back to the screen via the SpaceSwitcher.
      container.read(spaceSelectionProvider.notifier)
        ..selectOrg(orgId: 'org-1', orgName: 'Default Organization')
        ..selectSpace(spaceId: 'sp-1', spaceName: 'Default Space');

      await tester.pumpWidget(
        _harness(container: container, router: _router()),
      );
      await tester.pumpAndSettle();

      // The space tile is rendered as a Card (selectable list).
      expect(find.text('Default Space'), findsOneWidget);
      // And no indefinite spinner.
      expect(find.byType(CircularProgressIndicator), findsNothing);
      // We did NOT auto-navigate away — still on the selector.
      expect(find.text('CHAT_PAGE'), findsNothing);
    },
  );
}
