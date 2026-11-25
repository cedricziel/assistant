import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/integrations/caller_roles.dart';
import 'package:assistant_app/features/integrations/integration_list_tile.dart';
import 'package:assistant_app/features/integrations/space_integrations_screen.dart';
import 'package:assistant_app/features/spaces/space_provider.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../unit/integrations/integration_fakes.dart';

class _Spaces extends SpacesNotifier {
  @override
  Future<List<SpaceSummary>> build() async => [
    spaceSummary('eng', name: 'Engineering'),
  ];
}

final _rows = [
  integrationSummary(
    'int_space',
    name: 'Sentry',
    layer: IntegrationLayer.space,
    spaceId: 'eng',
  ),
  integrationSummary('int_org', name: 'GitHub'),
];

Future<FakeIntegrationsApi> _pump(
  WidgetTester tester, {
  required String role,
  double width = 800,
}) async {
  tester.view.physicalSize = Size(width, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final api = FakeIntegrationsApi()
    ..onList = ((layer, spaceId) async =>
        layer == IntegrationLayer.org ? [_rows[1]] : _rows)
    ..onGet = ((id) async => integrationDetail(id))
    ..onPutAccess = ((id, spaceId, mode) async {});
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        apiClientProvider.overrideWithValue(
          FakeApiClient(integrationsApi: api),
        ),
        spacesProvider.overrideWith(_Spaces.new),
        callerRolesProvider.overrideWith(
          (ref) => CallerRoles(
            spaces: [spaceSummary('eng', name: 'Engineering')],
            roles: {'eng': role},
          ),
        ),
      ],
      child: const MaterialApp(home: SpaceIntegrationsScreen(spaceId: 'eng')),
    ),
  );
  await tester.pumpAndSettle();
  return api;
}

void main() {
  group('SpaceIntegrationsScreen', () {
    testWidgets('titles the screen with the space', (tester) async {
      await _pump(tester, role: 'space_admin');

      expect(find.text('Engineering integrations'), findsOneWidget);
    });

    testWidgets('space admins manage their own and opt out of org ones', (
      tester,
    ) async {
      final api = await _pump(tester, role: 'space_admin');

      expect(find.byType(IntegrationListTile), findsNWidgets(2));
      // The space's own integration: enable switch and remove.
      expect(find.byTooltip('Remove'), findsOneWidget);
      // The org one: an opt-out switch for this space.
      expect(find.text('Use in this space'), findsOneWidget);

      await tester.tap(
        find.descendant(
          of: find.widgetWithText(IntegrationListTile, 'GitHub'),
          matching: find.byType(Switch),
        ),
      );
      await tester.pumpAndSettle();

      expect(api.calls, contains('put-access int_org eng optOut'));
      expect(find.text('Add integration'), findsOneWidget);
    });

    testWidgets('members get a read-only view', (tester) async {
      await _pump(tester, role: 'member');

      expect(find.byType(IntegrationListTile), findsNWidgets(2));
      expect(find.byType(Switch), findsNothing);
      expect(find.byTooltip('Remove'), findsNothing);
      expect(find.text('Add integration'), findsNothing);
      expect(
        find.text('Only space admins can change these integrations.'),
        findsOneWidget,
      );
    });

    testWidgets('an empty space says so', (tester) async {
      tester.view.physicalSize = const Size(800, 1200);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final api = FakeIntegrationsApi()
        ..onList = ((layer, spaceId) async => []);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            apiClientProvider.overrideWithValue(
              FakeApiClient(integrationsApi: api),
            ),
            spacesProvider.overrideWith(_Spaces.new),
            callerRolesProvider.overrideWith(
              (ref) => CallerRoles(
                spaces: [spaceSummary('eng')],
                roles: const {'eng': 'space_admin'},
              ),
            ),
          ],
          child: const MaterialApp(
            home: SpaceIntegrationsScreen(spaceId: 'eng'),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('No integrations in this space'), findsOneWidget);
    });

    testWidgets('says so and retries when the role cannot be read', (
      tester,
    ) async {
      var attempts = 0;
      tester.view.physicalSize = const Size(800, 1200);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final api = FakeIntegrationsApi()
        ..onList = ((layer, spaceId) async => [])
        ..onGet = ((id) async => integrationDetail(id));
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            apiClientProvider.overrideWithValue(
              FakeApiClient(integrationsApi: api),
            ),
            spacesProvider.overrideWith(_Spaces.new),
            callerRolesProvider.overrideWith((ref) async {
              if (attempts++ == 0) throw dioError(500);
              return CallerRoles(
                spaces: [spaceSummary('eng')],
                roles: const {'eng': 'space_admin'},
              );
            }),
          ],
          child: const MaterialApp(
            home: SpaceIntegrationsScreen(spaceId: 'eng'),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Could not check your role.'), findsOneWidget);
      expect(find.text('Add integration'), findsNothing);

      await tester.tap(find.text('Retry'));
      await tester.pumpAndSettle();

      expect(find.text('Add integration'), findsOneWidget);
    });

    testWidgets('fits a narrow phone without overflow', (tester) async {
      await _pump(tester, role: 'space_admin', width: 360);

      expect(tester.takeException(), isNull);
    });
  });
}
