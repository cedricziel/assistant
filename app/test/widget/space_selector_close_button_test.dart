// SpaceSelectorScreen MUST have a discoverable close affordance that returns
// to /chat without modifying spaceSelectionProvider.
//
// Spec: openspec/changes/space-selector-usability/specs/space-selector-usability/spec.md
//   "Space selector has a discoverable close affordance"

import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/spaces/space_provider.dart';
import 'package:assistant_app/features/spaces/space_selector_screen.dart';

OrgSummary _org(String id, String name) => OrgSummary(
  (b) => b
    ..authMode = 'password'
    ..id = id
    ..name = name
    ..slug = id,
);

SpaceSummary _space(String id, String name) => SpaceSummary(
  (b) => b
    ..id = id
    ..name = name
    ..slug = id,
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
    ref.watch(spaceSelectionProvider);
    return _spaces;
  }
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
  testWidgets('close affordance is visible on every entry', (tester) async {
    final container = ProviderContainer(
      overrides: [
        orgsProvider.overrideWith(
          () => _StaticOrgsNotifier([_org('org-1', 'Default')]),
        ),
        spacesProvider.overrideWith(
          () => _StaticSpacesNotifier([_space('sp-1', 'Default Space')]),
        ),
      ],
    );
    addTearDown(container.dispose);

    container.read(spaceSelectionProvider.notifier)
      ..selectOrg(orgId: 'org-1', orgName: 'Default')
      ..selectSpace(spaceId: 'sp-1', spaceName: 'Default Space');

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp.router(routerConfig: _router()),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('space-selector-close')),
      findsOneWidget,
      reason: 'Close affordance must be visible on /spaces',
    );
  });

  testWidgets('tapping close returns to /chat without changing selection', (
    tester,
  ) async {
    final container = ProviderContainer(
      overrides: [
        orgsProvider.overrideWith(
          () => _StaticOrgsNotifier([_org('org-1', 'Default')]),
        ),
        spacesProvider.overrideWith(
          () => _StaticSpacesNotifier([_space('sp-1', 'Default Space')]),
        ),
      ],
    );
    addTearDown(container.dispose);

    container.read(spaceSelectionProvider.notifier)
      ..selectOrg(orgId: 'org-1', orgName: 'Default')
      ..selectSpace(spaceId: 'sp-1', spaceName: 'Default Space');

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp.router(routerConfig: _router()),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('space-selector-close')));
    await tester.pumpAndSettle();

    expect(find.text('CHAT_PAGE'), findsOneWidget);
    final selection = container.read(spaceSelectionProvider);
    expect(selection.orgId, 'org-1');
    expect(selection.spaceId, 'sp-1');
  });
}
