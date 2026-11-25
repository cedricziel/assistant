import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/account/account_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/integrations/connections_screen.dart';
import 'package:assistant_app/features/integrations/integration_list_tile.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../unit/integrations/integration_fakes.dart';

Future<FakeIntegrationsApi> _pump(
  WidgetTester tester,
  List<IntegrationSummary> items, {
  double width = 800,
}) async {
  tester.view.physicalSize = Size(width, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final api = FakeIntegrationsApi()..onList = ((layer, spaceId) async => items);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        apiClientProvider.overrideWithValue(
          FakeApiClient(integrationsApi: api),
        ),
        currentUserIdProvider.overrideWith((ref) => 'me'),
      ],
      child: const MaterialApp(home: ConnectionsScreen()),
    ),
  );
  await tester.pumpAndSettle();
  return api;
}

void main() {
  group('ConnectionsScreen', () {
    testWidgets('lists personal and per-user integrations', (tester) async {
      await _pump(tester, [
        integrationSummary(
          'int_mine',
          name: 'Notion',
          layer: IntegrationLayer.user,
          ownerUserId: 'me',
          mode: CredentialMode.perUser,
          status: IntegrationStatus.needsConnection,
        ),
        integrationSummary(
          'int_org',
          name: 'Linear',
          mode: CredentialMode.perUser,
          status: IntegrationStatus.needsConnection,
        ),
        integrationSummary('int_shared', name: 'Shared one'),
      ]);

      expect(find.byType(IntegrationListTile), findsNWidgets(2));
      expect(find.text('Notion'), findsOneWidget);
      expect(find.text('Linear'), findsOneWidget);
      expect(find.text('Shared one'), findsNothing);
      expect(find.widgetWithText(FilledButton, 'Connect'), findsNWidgets(2));
    });

    testWidgets('only personal integrations can be switched or removed', (
      tester,
    ) async {
      await _pump(tester, [
        integrationSummary(
          'int_mine',
          layer: IntegrationLayer.user,
          ownerUserId: 'me',
          mode: CredentialMode.perUser,
        ),
        integrationSummary('int_org', mode: CredentialMode.perUser),
      ]);

      expect(find.byType(Switch), findsOneWidget);
      expect(find.byTooltip('Remove'), findsOneWidget);
    });

    testWidgets('an empty list points to the catalog', (tester) async {
      await _pump(tester, []);

      expect(find.text('No connections yet'), findsOneWidget);
      expect(find.text('Browse the catalog'), findsOneWidget);
    });

    testWidgets('fits a narrow phone without overflow', (tester) async {
      await _pump(tester, [
        integrationSummary(
          'int_mine',
          name: 'A very long personal integration name',
          layer: IntegrationLayer.user,
          ownerUserId: 'me',
          mode: CredentialMode.perUser,
          status: IntegrationStatus.needsConnection,
        ),
      ], width: 360);

      expect(tester.takeException(), isNull);
    });
  });
}
