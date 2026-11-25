import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/integrations/catalog_browser_screen.dart';
import 'package:assistant_app/features/integrations/catalog_entry_tile.dart';
import 'package:assistant_app/features/integrations/catalog_view_model.dart';
import 'package:built_collection/built_collection.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

class _StubViewModel extends CatalogViewModel {
  _StubViewModel(this._initial);

  final Future<CatalogState> _initial;
  final queries = <String>[];
  final selectedCategories = <String?>[];
  var retries = 0;
  var loadMores = 0;
  var clears = 0;

  @override
  Future<CatalogState> build() => _initial;

  @override
  void setQuery(String query) => queries.add(query);

  @override
  Future<void> selectCategory(String? category) async =>
      selectedCategories.add(category);

  @override
  Future<void> retry() async => retries++;

  @override
  Future<void> loadMore() async => loadMores++;

  @override
  Future<void> clearFilters() async => clears++;
}

CatalogEntrySummary _entry(
  String id, {
  CatalogAuthResponse? auth,
  String? description,
}) {
  return CatalogEntrySummary(
    (b) => b
      ..id = 'curated/$id'
      ..name = id
      ..description = description ?? 'About $id'
      ..domain = '$id.example.com'
      ..connectUrl = 'https://$id.example.com/mcp'
      ..categories = ListBuilder<String>(['dev'])
      ..auth = (auth ?? CatalogAuthResponse((a) => a..kind = .none))
          .toBuilder(),
  );
}

Future<_StubViewModel> _pump(
  WidgetTester tester,
  FutureOr<CatalogState> state, {
  Size size = const Size(800, 1200),
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final stub = _StubViewModel(Future.value(state));
  await tester.pumpWidget(
    ProviderScope(
      overrides: [catalogViewModelProvider.overrideWith(() => stub)],
      child: const MaterialApp(home: CatalogBrowserScreen()),
    ),
  );
  await tester.pumpAndSettle();
  return stub;
}

void main() {
  group('CatalogBrowserScreen', () {
    testWidgets('shows a spinner while the first page loads', (tester) async {
      final stub = _StubViewModel(Completer<CatalogState>().future);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [catalogViewModelProvider.overrideWith(() => stub)],
          child: const MaterialApp(home: CatalogBrowserScreen()),
        ),
      );

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
    });

    testWidgets('lists one tile per entry', (tester) async {
      await _pump(
        tester,
        CatalogState(items: [_entry('linear'), _entry('github')]),
      );

      expect(find.byType(CatalogEntryTile), findsNWidgets(2));
      expect(find.text('linear'), findsOneWidget);
      expect(find.text('github'), findsOneWidget);
    });

    testWidgets('forwards typed text to the view model', (tester) async {
      final stub = await _pump(tester, CatalogState(items: [_entry('linear')]));

      await tester.enterText(find.byType(TextField), 'lin');

      expect(stub.queries, ['lin']);
    });

    testWidgets('renders a chip per category and filters on tap', (
      tester,
    ) async {
      final stub = await _pump(
        tester,
        CatalogState(items: [_entry('linear')], categories: ['chat', 'dev']),
      );

      expect(find.widgetWithText(ChoiceChip, 'chat'), findsOneWidget);
      await tester.tap(find.widgetWithText(ChoiceChip, 'dev'));

      expect(stub.selectedCategories, ['dev']);
    });

    testWidgets('marks the active category chip as selected', (tester) async {
      await _pump(
        tester,
        CatalogState(
          items: [_entry('linear')],
          categories: ['chat', 'dev'],
          category: 'dev',
        ),
      );

      final chip = tester.widget<ChoiceChip>(
        find.widgetWithText(ChoiceChip, 'dev'),
      );
      expect(chip.selected, isTrue);
    });

    testWidgets('explains an empty catalog', (tester) async {
      await _pump(tester, const CatalogState());

      expect(find.text('No integrations available'), findsOneWidget);
      expect(find.text('Clear filters'), findsNothing);
    });

    testWidgets('explains an empty search and offers to clear it', (
      tester,
    ) async {
      final stub = await _pump(tester, const CatalogState(query: 'zzz'));

      expect(find.text('No integrations match your search'), findsOneWidget);
      await tester.tap(find.text('Clear filters'));

      expect(stub.clears, 1);
    });

    testWidgets('shows the error with a working retry', (tester) async {
      final stub = await _pump(tester, const CatalogState(error: 'boom'));

      expect(find.textContaining('boom'), findsOneWidget);
      await tester.tap(find.text('Retry'));

      expect(stub.retries, 1);
    });

    testWidgets('offers load more only when a next page exists', (
      tester,
    ) async {
      final stub = await _pump(
        tester,
        CatalogState(items: [_entry('linear')], nextCursor: 'c1'),
      );

      await tester.tap(find.text('Load more'));

      expect(stub.loadMores, 1);
    });

    testWidgets('hides load more on the last page', (tester) async {
      await _pump(tester, CatalogState(items: [_entry('linear')]));

      expect(find.text('Load more'), findsNothing);
    });

    testWidgets('says when loading more failed', (tester) async {
      await _pump(
        tester,
        CatalogState(
          items: [_entry('linear')],
          nextCursor: 'c1',
          loadMoreFailed: true,
        ),
      );

      expect(find.text('Could not load more. Try again'), findsOneWidget);
    });

    testWidgets('lays out on a phone-width screen', (tester) async {
      await _pump(
        tester,
        CatalogState(items: [_entry('linear')], categories: ['dev']),
        size: const Size(390, 800),
      );

      expect(tester.takeException(), isNull);
      expect(find.byType(CatalogEntryTile), findsOneWidget);
    });
  });

  group('entry detail', () {
    testWidgets('opens a sheet with the entry details', (tester) async {
      await _pump(
        tester,
        CatalogState(
          items: [
            _entry(
              'linear',
              description: 'Issue tracking for product teams',
              auth: CatalogAuthResponse(
                (a) => a
                  ..kind = .header
                  ..headerName = 'Authorization'
                  ..valuePrefix = 'Bearer '
                  ..placeholder = 'pat',
              ),
            ),
          ],
        ),
      );

      await tester.tap(find.byType(CatalogEntryTile));
      await tester.pumpAndSettle();

      expect(find.text('Issue tracking for product teams'), findsWidgets);
      expect(find.text('linear.example.com'), findsWidgets);
      expect(find.text('https://linear.example.com/mcp'), findsOneWidget);
      expect(
        find.text(
          'Needs an API key, sent in the Authorization header as Bearer {pat}.',
        ),
        findsOneWidget,
      );
    });

    testWidgets('describes OAuth requirements', (tester) async {
      await _pump(
        tester,
        CatalogState(
          items: [
            _entry(
              'linear',
              auth: CatalogAuthResponse((a) => a..kind = .oauth),
            ),
          ],
        ),
      );

      await tester.tap(find.byType(CatalogEntryTile));
      await tester.pumpAndSettle();

      expect(find.text('Sign in with OAuth when you connect.'), findsOneWidget);
    });

    testWidgets('shows a disabled Install button that says "Coming soon"', (
      tester,
    ) async {
      await _pump(tester, CatalogState(items: [_entry('linear')]));

      await tester.tap(find.byType(CatalogEntryTile));
      await tester.pumpAndSettle();

      final install = find.widgetWithText(FilledButton, 'Install');
      expect(install, findsOneWidget);
      expect(tester.widget<FilledButton>(install).onPressed, isNull);
      expect(find.byTooltip('Coming soon'), findsOneWidget);
    });
  });
}
