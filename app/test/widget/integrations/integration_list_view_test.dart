import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/integrations/caller_roles.dart';
import 'package:assistant_app/features/integrations/integration_list_tile.dart';
import 'package:assistant_app/features/integrations/integration_list_view.dart';
import 'package:assistant_app/features/integrations/integration_list_view_model.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../unit/integrations/integration_fakes.dart';

const _manage = IntegrationRowAccess(toggle: true, delete: true);

Future<FakeIntegrationsApi> _pump(
  WidgetTester tester,
  FakeIntegrationsApi api, {
  IntegrationListScope scope = const IntegrationListScope.org(),
  IntegrationRowAccess Function(IntegrationSummary)? access,
  Widget? emptyAction,
}) async {
  tester.view.physicalSize = const Size(800, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        apiClientProvider.overrideWithValue(
          FakeApiClient(integrationsApi: api),
        ),
        currentUserIdProvider.overrideWith((ref) => 'me'),
      ],
      child: MaterialApp(
        home: Scaffold(
          body: IntegrationListView(
            scope: scope,
            access: access ?? (_) => _manage,
            emptyTitle: 'No integrations',
            emptyMessage: 'Nothing installed yet.',
            emptyAction: emptyAction,
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return api;
}

FakeIntegrationsApi _api(List<IntegrationSummary> items) =>
    FakeIntegrationsApi()..onList = ((layer, spaceId) async => items);

void main() {
  group('IntegrationListView', () {
    testWidgets('shows a spinner while loading', (tester) async {
      final gate = Completer<List<IntegrationSummary>>();
      final api = FakeIntegrationsApi()
        ..onList = ((layer, spaceId) => gate.future);
      tester.view.physicalSize = const Size(800, 1200);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            apiClientProvider.overrideWithValue(
              FakeApiClient(integrationsApi: api),
            ),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: IntegrationListView(
                scope: const IntegrationListScope.org(),
                access: (_) => _manage,
                emptyTitle: 'No integrations',
                emptyMessage: 'Nothing installed yet.',
              ),
            ),
          ),
        ),
      );
      await tester.pump();

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      gate.complete([]);
      await tester.pumpAndSettle();
    });

    testWidgets('lists integrations', (tester) async {
      await _pump(
        tester,
        _api([
          integrationSummary('int_a', name: 'Linear'),
          integrationSummary('int_b', name: 'GitHub'),
        ]),
      );

      expect(find.byType(IntegrationListTile), findsNWidgets(2));
      expect(find.text('Linear'), findsOneWidget);
      expect(find.text('GitHub'), findsOneWidget);
    });

    testWidgets('shows the empty state with its action', (tester) async {
      await _pump(
        tester,
        _api([]),
        emptyAction: const Text('Browse the catalog'),
      );

      expect(find.text('No integrations'), findsOneWidget);
      expect(find.text('Nothing installed yet.'), findsOneWidget);
      expect(find.text('Browse the catalog'), findsOneWidget);
    });

    testWidgets('shows an error and retries', (tester) async {
      var attempts = 0;
      final api = FakeIntegrationsApi()
        ..onList = ((layer, spaceId) async {
          if (attempts++ == 0) throw dioError(500);
          return [integrationSummary('int_a', name: 'Linear')];
        });
      await _pump(tester, api);

      expect(
        find.textContaining('Failed to load integrations'),
        findsOneWidget,
      );
      await tester.tap(find.widgetWithText(FilledButton, 'Retry'));
      await tester.pumpAndSettle();

      expect(find.text('Linear'), findsOneWidget);
    });

    testWidgets('toggling the switch disables the integration', (tester) async {
      final api = _api([integrationSummary('int_a')])
        ..onUpdate = ((id, r) async => integrationDetail(
          id,
          enabled: false,
          status: IntegrationStatus.disabled,
        ));
      await _pump(tester, api);

      await tester.tap(find.byType(Switch));
      await tester.pumpAndSettle();

      expect(api.calls, contains('patch int_a enabled=false'));
      expect(find.text('Disabled'), findsOneWidget);
    });

    testWidgets('a conflict refreshes the list and tells the user', (
      tester,
    ) async {
      final api = _api([integrationSummary('int_a')])
        ..onUpdate = ((id, r) async => throw dioError(409));
      await _pump(tester, api);

      await tester.tap(find.byType(Switch));
      await tester.pumpAndSettle();

      expect(find.textContaining('changed by someone else'), findsOneWidget);
      expect(
        api.calls.where((c) => c.startsWith('list')),
        hasLength(2),
        reason: 'listed again after the conflict',
      );
    });

    testWidgets('remove asks for confirmation before deleting', (tester) async {
      final api = _api([integrationSummary('int_a', name: 'Linear')])
        ..onDelete = ((id) async {});
      await _pump(tester, api);

      await tester.tap(find.byTooltip('Remove'));
      await tester.pumpAndSettle();
      expect(find.text('Remove Linear?'), findsOneWidget);
      expect(api.calls, isNot(contains('delete int_a')));

      await tester.tap(find.text('Remove').last);
      await tester.pumpAndSettle();

      expect(api.calls, contains('delete int_a'));
      expect(find.byType(IntegrationListTile), findsNothing);
    });

    testWidgets('cancelling the confirmation keeps the integration', (
      tester,
    ) async {
      final api = _api([integrationSummary('int_a', name: 'Linear')])
        ..onDelete = ((id) async {});
      await _pump(tester, api);

      await tester.tap(find.byTooltip('Remove'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(api.calls, isNot(contains('delete int_a')));
      expect(find.text('Linear'), findsOneWidget);
    });

    testWidgets('a read-only row has no controls', (tester) async {
      await _pump(
        tester,
        _api([integrationSummary('int_a')]),
        access: (_) => IntegrationRowAccess.none,
      );

      expect(find.byType(Switch), findsNothing);
      expect(find.byTooltip('Remove'), findsNothing);
    });

    testWidgets('the space scope offers opt-out for org integrations', (
      tester,
    ) async {
      final api = FakeIntegrationsApi()
        ..onList = ((layer, spaceId) async => [
          integrationSummary('int_org'),
          integrationSummary(
            'int_space',
            layer: IntegrationLayer.space,
            spaceId: 'eng',
          ),
        ])
        ..onPutAccess = ((id, spaceId, mode) async {});
      await _pump(
        tester,
        api,
        scope: const IntegrationListScope.space('eng'),
        access: (i) => i.layer == IntegrationLayer.org
            ? const IntegrationRowAccess(optOut: true)
            : _manage,
      );

      expect(find.text('Use in this space'), findsOneWidget);
      await tester.tap(find.byTooltip('Remove'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      await tester.tap(
        find.descendant(
          of: find.byType(IntegrationListTile).first,
          matching: find.byType(Switch),
        ),
      );
      await tester.pumpAndSettle();

      expect(api.calls, contains('put-access int_org eng optOut'));
    });
  });
}
