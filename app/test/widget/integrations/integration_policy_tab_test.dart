import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/integrations/catalog_view_model.dart';
import 'package:assistant_app/features/integrations/integration_policy_tab.dart';
import 'package:built_collection/built_collection.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../unit/integrations/integration_fakes.dart';

class _StubCatalog extends CatalogViewModel {
  _StubCatalog(this._items);
  final List<CatalogEntrySummary> _items;

  @override
  Future<CatalogState> build() async => CatalogState(items: _items);

  @override
  void setQuery(String query) {}
}

CatalogEntrySummary _entry(String slug, String name) => CatalogEntrySummary(
  (b) => b
    ..id = 'curated/$slug'
    ..name = name
    ..description = 'About $name'
    ..domain = '$slug.example.com'
    ..connectUrl = 'https://$slug.example.com/mcp'
    ..categories = ListBuilder<String>()
    ..installableLayers = ListBuilder<IntegrationLayer>()
    ..auth = CatalogAuthResponse((a) => a..kind = .none).toBuilder(),
);

Future<FakeIntegrationsApi> _pump(
  WidgetTester tester, {
  IntegrationPolicyResponse? current,
  Future<IntegrationPolicyResponse> Function()? load,
  double width = 800,
}) async {
  tester.view.physicalSize = Size(width, 1600);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final api = FakeIntegrationsApi()
    ..onGetPolicy = (load ?? () async => current ?? policy());
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        apiClientProvider.overrideWithValue(
          FakeApiClient(integrationsApi: api),
        ),
        catalogViewModelProvider.overrideWith(
          () => _StubCatalog([
            _entry('linear', 'Linear'),
            _entry('github', 'GitHub'),
          ]),
        ),
      ],
      child: const MaterialApp(home: Scaffold(body: IntegrationPolicyTab())),
    ),
  );
  await tester.pumpAndSettle();
  return api;
}

Finder _save() => find.widgetWithText(FilledButton, 'Save policy');

void main() {
  group('IntegrationPolicyTab', () {
    testWidgets('shows a spinner while loading', (tester) async {
      final gate = Completer<IntegrationPolicyResponse>();
      tester.view.physicalSize = const Size(800, 1600);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final api = FakeIntegrationsApi()..onGetPolicy = (() => gate.future);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            apiClientProvider.overrideWithValue(
              FakeApiClient(integrationsApi: api),
            ),
          ],
          child: const MaterialApp(
            home: Scaffold(body: IntegrationPolicyTab()),
          ),
        ),
      );
      await tester.pump();

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      gate.complete(policy());
      await tester.pumpAndSettle();
    });

    testWidgets('shows an error and retries', (tester) async {
      var attempts = 0;
      await _pump(
        tester,
        load: () async {
          if (attempts++ == 0) throw dioError(500);
          return policy();
        },
      );
      expect(find.textContaining('Failed to load the policy'), findsOneWidget);

      await tester.tap(find.widgetWithText(FilledButton, 'Retry'));
      await tester.pumpAndSettle();

      expect(find.text('Catalog'), findsOneWidget);
    });

    testWidgets('shows the current policy', (tester) async {
      await _pump(
        tester,
        current: policy(customUrls: true, userIntegrations: false),
      );

      final chip = tester.widget<ChoiceChip>(
        find.widgetWithText(ChoiceChip, 'All catalog entries'),
      );
      expect(chip.selected, isTrue);
      final switches = tester.widgetList<Switch>(find.byType(Switch)).toList();
      expect(switches.map((s) => s.value), [true, false]);
    });

    testWidgets('Save is disabled until something changes', (tester) async {
      await _pump(tester);

      expect(tester.widget<FilledButton>(_save()).onPressed, isNull);

      await tester.tap(find.byType(Switch).first);
      await tester.pump();

      expect(tester.widget<FilledButton>(_save()).onPressed, isNotNull);
    });

    testWidgets('saving sends the edited policy', (tester) async {
      final api = await _pump(tester);
      UpdateIntegrationPolicyRequest? sent;
      api.onPutPolicy = (request) async {
        sent = request;
        return policy(customUrls: true);
      };

      await tester.tap(find.byType(Switch).first);
      await tester.pump();
      await tester.tap(_save());
      await tester.pumpAndSettle();

      expect(sent!.allowCustomUrls, isTrue);
      expect(sent!.allowUserIntegrations, isTrue);
      expect(sent!.catalogMode, CatalogMode.all);
      expect(find.text('Policy saved'), findsOneWidget);
      expect(tester.widget<FilledButton>(_save()).onPressed, isNull);
    });

    testWidgets('a 403 shows why saving failed', (tester) async {
      final api = await _pump(tester);
      api.onPutPolicy = (request) async => throw dioError(403);

      await tester.tap(find.byType(Switch).first);
      await tester.pump();
      await tester.tap(_save());
      await tester.pumpAndSettle();

      expect(find.textContaining('does not allow'), findsOneWidget);
    });

    testWidgets('the allowlist editor appears only in allowlist mode', (
      tester,
    ) async {
      await _pump(tester);
      expect(find.text('Allowed entries'), findsNothing);

      await tester.tap(
        find.widgetWithText(ChoiceChip, 'Only selected entries'),
      );
      await tester.pumpAndSettle();

      expect(find.text('Allowed entries'), findsOneWidget);
      expect(find.textContaining('Nothing is allowed yet'), findsOneWidget);
    });

    testWidgets('entries are picked from the catalog and saved', (
      tester,
    ) async {
      final api = await _pump(tester);
      UpdateIntegrationPolicyRequest? sent;
      api.onPutPolicy = (request) async {
        sent = request;
        return policy(mode: CatalogMode.allowlist, allowed: ['curated/linear']);
      };

      await tester.tap(
        find.widgetWithText(ChoiceChip, 'Only selected entries'),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Add from catalog'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(CheckboxListTile, 'Linear'));
      await tester.pump();
      await tester.tap(find.widgetWithText(FilledButton, 'Done'));
      await tester.pumpAndSettle();

      expect(find.widgetWithText(InputChip, 'linear'), findsOneWidget);

      await tester.tap(_save());
      await tester.pumpAndSettle();

      expect(sent!.catalogMode, CatalogMode.allowlist);
      expect(sent!.allowedCatalogIds!.toList(), ['curated/linear']);
    });

    testWidgets('an allowed entry can be removed again', (tester) async {
      await _pump(
        tester,
        current: policy(
          mode: CatalogMode.allowlist,
          allowed: ['curated/linear'],
        ),
      );

      await tester.tap(
        find.descendant(
          of: find.widgetWithText(InputChip, 'linear'),
          matching: find.byIcon(Icons.cancel),
        ),
      );
      await tester.pump();

      expect(find.widgetWithText(InputChip, 'linear'), findsNothing);
      expect(tester.widget<FilledButton>(_save()).onPressed, isNotNull);
    });

    testWidgets('fits a narrow phone without overflow', (tester) async {
      await _pump(
        tester,
        current: policy(
          mode: CatalogMode.allowlist,
          allowed: ['curated/linear', 'curated/github'],
        ),
        width: 360,
      );

      expect(tester.takeException(), isNull);
    });
  });
}
