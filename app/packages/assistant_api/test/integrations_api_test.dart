import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for IntegrationsApi
void main() {
  final instance = AssistantApi().getIntegrationsApi();

  group(IntegrationsApi, () {
    // `GET /api/integrations/catalog/{id}` — get one catalog entry.
    //
    //Future<CatalogEntrySummary> getCatalogEntry(String id) async
    test('test getCatalogEntry', () async {
      // TODO
    });

    // `GET /api/integrations/catalog` — search the integrations catalog.
    //
    //Future<CatalogEntryListResponse> listCatalogEntries({ String q, String category, int limit, String cursor }) async
    test('test listCatalogEntries', () async {
      // TODO
    });
  });
}
