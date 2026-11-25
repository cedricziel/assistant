import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/integrations/auth_kind_badge.dart';
import 'package:assistant_app/features/integrations/catalog_entry_tile.dart';
import 'package:built_collection/built_collection.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

CatalogEntrySummary _entry({
  String name = 'Linear',
  String? iconUrl,
  CatalogAuthKindResponse kind = CatalogAuthKindResponse.oauth,
}) {
  return CatalogEntrySummary(
    (b) => b
      ..id = 'curated/linear-app-mcp'
      ..name = name
      ..description = 'Issue tracking for product teams'
      ..domain = 'linear.app'
      ..connectUrl = 'https://mcp.linear.app/mcp'
      ..iconUrl = iconUrl
      ..categories = ListBuilder<String>(['project-management'])
      ..auth = CatalogAuthResponse((a) => a..kind = kind).toBuilder(),
  );
}

Future<void> _pump(
  WidgetTester tester,
  CatalogEntrySummary entry, {
  VoidCallback? onTap,
}) {
  return tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: CatalogEntryTile(entry: entry, onTap: onTap ?? () {}),
      ),
    ),
  );
}

void main() {
  group('CatalogEntryTile', () {
    testWidgets('shows name, domain, description and auth badge', (
      tester,
    ) async {
      await _pump(tester, _entry());

      expect(find.text('Linear'), findsOneWidget);
      expect(find.text('linear.app'), findsOneWidget);
      expect(find.text('Issue tracking for product teams'), findsOneWidget);
      expect(find.byType(AuthKindBadge), findsOneWidget);
      expect(find.text('OAuth'), findsOneWidget);
    });

    testWidgets('invokes onTap when tapped', (tester) async {
      var taps = 0;
      await _pump(tester, _entry(), onTap: () => taps++);

      await tester.tap(find.byType(CatalogEntryTile));

      expect(taps, 1);
    });

    testWidgets('loads the icon from icon_url', (tester) async {
      await _pump(tester, _entry(iconUrl: 'https://cdn.example.com/l.png'));

      final image = tester.widget<Image>(find.byType(Image));
      expect(
        (image.image as NetworkImage).url,
        'https://cdn.example.com/l.png',
      );
    });

    testWidgets('falls back to the initial when there is no icon_url', (
      tester,
    ) async {
      await _pump(tester, _entry());

      expect(find.byType(Image), findsNothing);
      expect(find.text('L'), findsOneWidget);
    });

    testWidgets('falls back to the initial when the icon fails to load', (
      tester,
    ) async {
      // flutter_test answers every HTTP request with a 400, so the image
      // errors out and the errorBuilder takes over.
      await _pump(tester, _entry(iconUrl: 'https://cdn.example.com/l.png'));
      await tester.pumpAndSettle();

      expect(find.text('L'), findsOneWidget);
    });
  });
}
