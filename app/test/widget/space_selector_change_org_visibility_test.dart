// "Change organization" must hide for single-org users and show for multi-org users.
//
// Spec: openspec/changes/space-selector-usability/specs/space-selector-usability/spec.md
//   "Change organization hides for single-org users"

import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

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

void main() {
  testWidgets('single-org: "Change organization" is hidden', (tester) async {
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
        child: const MaterialApp(home: SpaceSelectorScreen()),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.text('Change organization'),
      findsNothing,
      reason: 'Single-org users should not see the loop trigger',
    );
  });

  testWidgets('multi-org: "Change organization" is visible', (tester) async {
    final container = ProviderContainer(
      overrides: [
        orgsProvider.overrideWith(
          () => _StaticOrgsNotifier([
            _org('org-1', 'Default'),
            _org('org-2', 'Other'),
          ]),
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
        child: const MaterialApp(home: SpaceSelectorScreen()),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.text('Change organization'),
      findsOneWidget,
      reason: 'Multi-org users should see the change-org button',
    );
  });
}
