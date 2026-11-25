import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/integrations/auth_kind_badge.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Future<void> _pump(WidgetTester tester, CatalogAuthKindResponse kind) {
  return tester.pumpWidget(
    MaterialApp(
      home: Scaffold(body: AuthKindBadge(kind: kind)),
    ),
  );
}

void main() {
  const labels = {
    CatalogAuthKindResponse.none: 'No auth',
    CatalogAuthKindResponse.header: 'API key',
    CatalogAuthKindResponse.oauth: 'OAuth',
    CatalogAuthKindResponse.mixed: 'API key or OAuth',
    CatalogAuthKindResponse.unknown: 'Auth unknown',
  };

  for (final entry in labels.entries) {
    testWidgets('labels ${entry.key.name} as "${entry.value}"', (tester) async {
      await _pump(tester, entry.key);

      expect(find.text(entry.value), findsOneWidget);
    });
  }

  testWidgets('treats a kind from a newer server as unknown', (tester) async {
    await _pump(tester, CatalogAuthKindResponse.unknownDefaultOpenApi);

    expect(find.text('Auth unknown'), findsOneWidget);
  });
}
