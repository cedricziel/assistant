import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/integrations/caller_roles.dart';
import 'package:assistant_app/features/integrations/integration_list_tile.dart';
import 'package:assistant_app/features/integrations/org_integrations_screen.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../unit/integrations/integration_fakes.dart';

Future<FakeIntegrationsApi> _pump(
  WidgetTester tester, {
  required FutureOr<CallerRoles> roles,
  List<IntegrationSummary> items = const [],
  double width = 800,
}) async {
  tester.view.physicalSize = Size(width, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final api = FakeIntegrationsApi()
    ..onList = ((layer, spaceId) async => items)
    ..onGetPolicy = (() async => policy());
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        apiClientProvider.overrideWithValue(
          FakeApiClient(integrationsApi: api),
        ),
        callerRolesProvider.overrideWith((ref) => roles),
      ],
      child: const MaterialApp(home: OrgIntegrationsScreen()),
    ),
  );
  await tester.pumpAndSettle();
  return api;
}

CallerRoles _admin() => CallerRoles(
  spaces: [spaceSummary('eng')],
  roles: const {'eng': 'org_admin'},
);

void main() {
  group('OrgIntegrationsScreen', () {
    testWidgets('org admins see the integrations and the policy tabs', (
      tester,
    ) async {
      final api = await _pump(
        tester,
        roles: _admin(),
        items: [integrationSummary('int_a', name: 'Linear')],
      );

      expect(find.widgetWithText(Tab, 'Integrations'), findsOneWidget);
      expect(find.widgetWithText(Tab, 'Policy'), findsOneWidget);
      expect(find.text('Linear'), findsOneWidget);
      expect(find.byType(Switch), findsOneWidget);
      expect(find.byTooltip('Remove'), findsOneWidget);
      expect(api.calls, contains('list layer=org space=null'));
    });

    testWidgets('the policy tab shows the policy editor', (tester) async {
      await _pump(tester, roles: _admin());

      await tester.tap(find.widgetWithText(Tab, 'Policy'));
      await tester.pumpAndSettle();

      expect(find.text('Allow custom server URLs'), findsOneWidget);
    });

    testWidgets('an empty org points to the catalog', (tester) async {
      await _pump(tester, roles: _admin());

      expect(find.text('No organization integrations'), findsOneWidget);
      expect(find.text('Browse the catalog'), findsOneWidget);
    });

    testWidgets('others are told only org admins can manage them', (
      tester,
    ) async {
      final api = await _pump(
        tester,
        roles: CallerRoles(
          spaces: [spaceSummary('eng')],
          roles: const {'eng': 'space_admin'},
        ),
      );

      expect(
        find.text(
          'Only organization admins can manage organization integrations.',
        ),
        findsOneWidget,
      );
      expect(find.byType(Tab), findsNothing);
      expect(api.calls, isEmpty);
    });

    testWidgets('shows a spinner while the role is checked', (tester) async {
      final gate = Completer<CallerRoles>();
      tester.view.physicalSize = const Size(800, 1200);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [callerRolesProvider.overrideWith((ref) => gate.future)],
          child: const MaterialApp(home: OrgIntegrationsScreen()),
        ),
      );
      await tester.pump();

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      gate.complete(_admin());
      await tester.pumpAndSettle();
    });

    testWidgets('fits a narrow phone without overflow', (tester) async {
      await _pump(
        tester,
        roles: _admin(),
        items: [
          integrationSummary(
            'int_a',
            name: 'A very long integration name',
            status: IntegrationStatus.needsConnection,
          ),
        ],
        width: 360,
      );

      expect(find.byType(IntegrationListTile), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  });
}
