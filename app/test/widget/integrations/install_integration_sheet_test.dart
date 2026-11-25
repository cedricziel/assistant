import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/integrations/install_integration_sheet.dart';
import 'package:assistant_app/features/integrations/install_integration_view_model.dart';
import 'package:built_collection/built_collection.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../unit/integrations/integration_fakes.dart';

CatalogEntrySummary _entry({
  CatalogAuthKindResponse kind = CatalogAuthKindResponse.oauth,
}) {
  return CatalogEntrySummary(
    (b) => b
      ..id = 'curated/linear-app-mcp'
      ..name = 'Linear'
      ..description = 'Issues'
      ..domain = 'linear.app'
      ..connectUrl = 'https://mcp.linear.app/mcp'
      ..categories = ListBuilder<String>()
      ..installableLayers = ListBuilder<IntegrationLayer>([
        IntegrationLayer.org,
        IntegrationLayer.space,
        IntegrationLayer.user,
      ])
      ..auth = CatalogAuthResponse((a) => a..kind = kind).toBuilder(),
  );
}

InstallArgs _args({
  CatalogEntrySummary? entry,
  List<IntegrationLayer> layers = const [
    IntegrationLayer.org,
    IntegrationLayer.space,
    IntegrationLayer.user,
  ],
  bool custom = false,
}) => InstallArgs(
  entry: custom ? null : (entry ?? _entry()),
  layers: layers,
  spaces: [
    spaceSummary('eng', name: 'Engineering'),
    spaceSummary('ops'),
  ],
);

Future<FakeIntegrationsApi> _pump(
  WidgetTester tester,
  InstallArgs args, {
  double width = 800,
}) async {
  tester.view.physicalSize = Size(width, 1800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final api = FakeIntegrationsApi();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        apiClientProvider.overrideWithValue(
          FakeApiClient(integrationsApi: api),
        ),
      ],
      child: MaterialApp(
        home: Scaffold(body: InstallIntegrationSheet(args: args)),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return api;
}

Finder _installButton() => find.widgetWithText(FilledButton, 'Install');

void main() {
  group('InstallIntegrationSheet', () {
    testWidgets('prefills name and slug and offers the allowed layers', (
      tester,
    ) async {
      await _pump(
        tester,
        _args(layers: [IntegrationLayer.org, IntegrationLayer.user]),
      );

      expect(find.text('Install Linear'), findsOneWidget);
      expect(
        find.widgetWithText(TextFormField, 'linear-app-mcp'),
        findsOneWidget,
      );
      expect(find.widgetWithText(TextFormField, 'Linear'), findsOneWidget);
      expect(
        find.widgetWithText(ChoiceChip, 'Whole organization'),
        findsOneWidget,
      );
      expect(find.widgetWithText(ChoiceChip, 'Just me'), findsOneWidget);
      expect(find.widgetWithText(ChoiceChip, 'One space'), findsNothing);
    });

    testWidgets('the space picker appears for the space layer', (tester) async {
      await _pump(tester, _args());
      expect(find.widgetWithText(ChoiceChip, 'Engineering'), findsNothing);

      await tester.tap(find.widgetWithText(ChoiceChip, 'One space'));
      await tester.pumpAndSettle();

      expect(find.widgetWithText(ChoiceChip, 'Engineering'), findsOneWidget);
      expect(find.widgetWithText(ChoiceChip, 'ops'), findsOneWidget);
    });

    testWidgets('credential mode is hidden for the personal layer', (
      tester,
    ) async {
      await _pump(tester, _args());
      expect(find.text('Who connects?'), findsNothing);

      await tester.tap(find.widgetWithText(ChoiceChip, 'Whole organization'));
      await tester.pumpAndSettle();

      expect(find.text('Who connects?'), findsOneWidget);
    });

    testWidgets('warns about shared OAuth sign-ins', (tester) async {
      await _pump(tester, _args(layers: [IntegrationLayer.org]));
      expect(find.textContaining('acts as one person'), findsNothing);

      await tester.tap(
        find.widgetWithText(ChoiceChip, 'One shared credential'),
      );
      await tester.pumpAndSettle();

      expect(find.textContaining('acts as one person'), findsOneWidget);
    });

    testWidgets('an invalid slug shows an error and disables Install', (
      tester,
    ) async {
      await _pump(tester, _args());

      await tester.enterText(
        find.widgetWithText(TextFormField, 'linear-app-mcp'),
        'Not Kebab',
      );
      await tester.pump();

      expect(find.textContaining('lower-case'), findsOneWidget);
      expect(tester.widget<FilledButton>(_installButton()).onPressed, isNull);
    });

    testWidgets('an entry with unknown auth asks the user to choose', (
      tester,
    ) async {
      await _pump(
        tester,
        _args(
          entry: _entry(kind: CatalogAuthKindResponse.unknownDefaultOpenApi),
        ),
      );

      expect(find.text('How does it sign in?'), findsOneWidget);
      expect(tester.widget<FilledButton>(_installButton()).onPressed, isNull);

      await tester.tap(find.widgetWithText(ChoiceChip, 'OAuth'));
      await tester.pumpAndSettle();

      expect(
        tester.widget<FilledButton>(_installButton()).onPressed,
        isNotNull,
      );
    });

    testWidgets('a known auth kind asks nothing', (tester) async {
      await _pump(tester, _args());

      expect(find.text('How does it sign in?'), findsNothing);
    });

    testWidgets('a custom integration asks for the URL', (tester) async {
      await _pump(tester, _args(custom: true));

      expect(find.text('Add custom integration'), findsOneWidget);
      expect(find.widgetWithText(TextFormField, 'Server URL'), findsOneWidget);
      expect(tester.widget<FilledButton>(_installButton()).onPressed, isNull);
    });

    testWidgets('installing shows the new integration and its status', (
      tester,
    ) async {
      final api = await _pump(tester, _args());
      api.onCreate = (request) async => integrationDetail(
        'int_new',
        name: 'Linear',
        layer: IntegrationLayer.user,
        status: IntegrationStatus.needsConnection,
      );

      await tester.tap(_installButton());
      await tester.pumpAndSettle();

      expect(find.text('Installed'), findsOneWidget);
      expect(find.text('Needs connection'), findsOneWidget);
      final connect = find.widgetWithText(FilledButton, 'Connect');
      expect(tester.widget<FilledButton>(connect).onPressed, isNull);
      expect(
        find.byTooltip('Credential setup arrives in a later update.'),
        findsOneWidget,
      );
    });

    testWidgets('shows a spinner while installing', (tester) async {
      final api = await _pump(tester, _args());
      final gate = Completer<IntegrationDetail>();
      api.onCreate = (request) => gate.future;

      await tester.tap(_installButton());
      await tester.pump();

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      gate.complete(integrationDetail('int_new'));
      await tester.pumpAndSettle();
    });

    testWidgets('a 403 shows the policy message', (tester) async {
      final api = await _pump(tester, _args());
      api.onCreate = (request) async => throw dioError(403);

      await tester.tap(_installButton());
      await tester.pumpAndSettle();

      expect(find.textContaining('does not allow'), findsOneWidget);
      expect(find.text('Installed'), findsNothing);
    });

    testWidgets('a 409 marks the slug field', (tester) async {
      final api = await _pump(tester, _args());
      api.onCreate = (request) async => throw dioError(409);

      await tester.tap(_installButton());
      await tester.pumpAndSettle();

      expect(find.textContaining('already used'), findsOneWidget);
    });

    testWidgets('a 422 shows what the server rejected', (tester) async {
      final api = await _pump(tester, _args());
      api.onCreate = (request) async =>
          throw dioError(422, body: {'error': 'connect_url must be https'});

      await tester.tap(_installButton());
      await tester.pumpAndSettle();

      expect(find.textContaining('connect_url must be https'), findsOneWidget);
    });

    testWidgets('fits a narrow phone without overflow', (tester) async {
      await _pump(tester, _args(), width: 360);

      expect(tester.takeException(), isNull);
    });
  });
}
