import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/spaces/space_provider.dart';
import 'package:assistant_app/features/spaces/space_selector_screen.dart';

GoRouter _router() => GoRouter(
  initialLocation: '/spaces',
  routes: [
    GoRoute(path: '/spaces', builder: (ctx, s) => const SpaceSelectorScreen()),
    GoRoute(path: '/chat', builder: (ctx, s) => const Scaffold()),
  ],
);

OrgSummary _makeOrg({required String id, required String name, String? slug}) {
  return OrgSummary(
    (b) => b
      ..id = id
      ..name = name
      ..slug = slug ?? id
      ..authMode = 'password',
  );
}

SpaceSummary _makeSpace({
  required String id,
  required String name,
  String? slug,
}) {
  return SpaceSummary(
    (b) => b
      ..id = id
      ..name = name
      ..slug = slug ?? id,
  );
}

void main() {
  group('SpaceSelectorScreen', () {
    group('org selection step', () {
      testWidgets('shows "Select organization" heading', (tester) async {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [orgsProvider.overrideWith(() => _StubOrgsNotifier([]))],
            child: MaterialApp.router(routerConfig: _router()),
          ),
        );
        await tester.pumpAndSettle();

        expect(find.text('Select organization'), findsOneWidget);
      });

      testWidgets('shows empty message when no orgs', (tester) async {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [orgsProvider.overrideWith(() => _StubOrgsNotifier([]))],
            child: MaterialApp.router(routerConfig: _router()),
          ),
        );
        await tester.pumpAndSettle();

        expect(find.textContaining('No organizations found'), findsOneWidget);
      });

      testWidgets('shows org list when multiple orgs', (tester) async {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              orgsProvider.overrideWith(
                () => _StubOrgsNotifier([
                  _makeOrg(id: 'o1', name: 'Acme Corp'),
                  _makeOrg(id: 'o2', name: 'Globex'),
                ]),
              ),
            ],
            child: MaterialApp.router(routerConfig: _router()),
          ),
        );
        await tester.pumpAndSettle();

        expect(find.text('Acme Corp'), findsOneWidget);
        expect(find.text('Globex'), findsOneWidget);
      });

      testWidgets('tapping org selects it and shows space step', (
        tester,
      ) async {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              orgsProvider.overrideWith(
                () => _StubOrgsNotifier([
                  _makeOrg(id: 'o1', name: 'Acme Corp'),
                  _makeOrg(id: 'o2', name: 'Globex'),
                ]),
              ),
              spacesProvider.overrideWith(() => _StubSpacesNotifier([])),
            ],
            child: MaterialApp.router(routerConfig: _router()),
          ),
        );
        await tester.pumpAndSettle();

        await tester.tap(find.text('Acme Corp'));
        await tester.pumpAndSettle();

        expect(find.text('Select space'), findsOneWidget);
        expect(find.textContaining('Acme Corp'), findsWidgets);
      });
    });

    group('space selection step', () {
      testWidgets('shows space list', (tester) async {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              spaceSelectionProvider.overrideWith(
                () => _PreselectedOrgNotifier('o1', 'Acme Corp'),
              ),
              spacesProvider.overrideWith(
                () => _StubSpacesNotifier([
                  _makeSpace(id: 's1', name: 'Engineering'),
                  _makeSpace(id: 's2', name: 'Marketing'),
                ]),
              ),
            ],
            child: MaterialApp.router(routerConfig: _router()),
          ),
        );
        await tester.pumpAndSettle();

        expect(find.text('Engineering'), findsOneWidget);
        expect(find.text('Marketing'), findsOneWidget);
      });

      testWidgets('shows empty message when no spaces', (tester) async {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              spaceSelectionProvider.overrideWith(
                () => _PreselectedOrgNotifier('o1', 'Acme Corp'),
              ),
              spacesProvider.overrideWith(() => _StubSpacesNotifier([])),
            ],
            child: MaterialApp.router(routerConfig: _router()),
          ),
        );
        await tester.pumpAndSettle();

        expect(find.textContaining('No spaces available'), findsOneWidget);
      });

      testWidgets('tapping space navigates to chat', (tester) async {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              spaceSelectionProvider.overrideWith(
                () => _PreselectedOrgNotifier('o1', 'Acme Corp'),
              ),
              spacesProvider.overrideWith(
                () => _StubSpacesNotifier([
                  _makeSpace(id: 's1', name: 'Engineering'),
                  _makeSpace(id: 's2', name: 'Marketing'),
                ]),
              ),
            ],
            child: MaterialApp.router(routerConfig: _router()),
          ),
        );
        await tester.pumpAndSettle();

        await tester.tap(find.text('Engineering'));
        await tester.pumpAndSettle();

        // Should have navigated to /chat.
        expect(find.byType(SpaceSelectorScreen), findsNothing);
        expect(find.byType(Scaffold), findsOneWidget);
      });

      testWidgets('back button returns to org selection', (tester) async {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              orgsProvider.overrideWith(
                () => _StubOrgsNotifier([
                  _makeOrg(id: 'o1', name: 'Acme Corp'),
                  _makeOrg(id: 'o2', name: 'Globex'),
                ]),
              ),
              spaceSelectionProvider.overrideWith(
                () => _PreselectedOrgNotifier('o1', 'Acme Corp'),
              ),
              spacesProvider.overrideWith(
                () => _StubSpacesNotifier([
                  _makeSpace(id: 's1', name: 'Engineering'),
                  _makeSpace(id: 's2', name: 'Marketing'),
                ]),
              ),
            ],
            child: MaterialApp.router(routerConfig: _router()),
          ),
        );
        await tester.pumpAndSettle();

        // Should be on space step.
        expect(find.text('Select space'), findsOneWidget);

        // Tap "Change organization".
        await tester.tap(find.text('Change organization'));
        await tester.pumpAndSettle();

        // Should be back on org step.
        expect(find.text('Select organization'), findsOneWidget);
      });
    });
  });
}

// -- Test stubs ---------------------------------------------------------------

/// Stub notifier that returns a fixed list of orgs.
class _StubOrgsNotifier extends AsyncNotifier<List<OrgSummary>>
    implements OrgsNotifier {
  _StubOrgsNotifier(this._orgs);
  final List<OrgSummary> _orgs;

  @override
  Future<List<OrgSummary>> build() async => _orgs;
}

/// Stub notifier that returns a fixed list of spaces.
class _StubSpacesNotifier extends AsyncNotifier<List<SpaceSummary>>
    implements SpacesNotifier {
  _StubSpacesNotifier(this._spaces);
  final List<SpaceSummary> _spaces;

  @override
  Future<List<SpaceSummary>> build() async => _spaces;
}

/// SpaceSelectionNotifier that starts with an org pre-selected.
class _PreselectedOrgNotifier extends SpaceSelectionNotifier {
  _PreselectedOrgNotifier(this._orgId, this._orgName);
  final String _orgId;
  final String _orgName;

  @override
  SpaceSelection build() => SpaceSelection(orgId: _orgId, orgName: _orgName);
}
