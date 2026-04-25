import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for CatalogApi
void main() {
  final instance = AssistantApi().getCatalogApi();

  group(CatalogApi, () {
    // `POST /api/orgs/{org_id}/spaces/{space_id}/subscriptions` — subscribe a space to a catalog item.
    //
    //Future<SubscriptionResponse> createSubscription(String orgId, String spaceId, CreateSubscriptionRequest createSubscriptionRequest) async
    test('test createSubscription', () async {
      // TODO
    });

    // `DELETE /api/orgs/{org_id}/catalog/{item_id}` — remove from catalog.
    //
    //Future deleteCatalogItem(String orgId, String itemId) async
    test('test deleteCatalogItem', () async {
      // TODO
    });

    // `DELETE /api/orgs/{org_id}/spaces/{space_id}/subscriptions/{sub_id}` — unsubscribe.
    //
    //Future deleteSubscription(String orgId, String spaceId, String subId) async
    test('test deleteSubscription', () async {
      // TODO
    });

    // `GET /api/orgs/{org_id}/catalog` — list all catalog items.
    //
    //Future<BuiltList<CatalogItemResponse>> listCatalog(String orgId) async
    test('test listCatalog', () async {
      // TODO
    });

    // `GET /api/orgs/{org_id}/catalog/{type}` — list catalog items by type.
    //
    //Future<BuiltList<CatalogItemResponse>> listCatalogByType(String orgId, String type) async
    test('test listCatalogByType', () async {
      // TODO
    });

    // `GET /api/orgs/{org_id}/spaces/{space_id}/subscriptions` — list subscriptions.
    //
    //Future<BuiltList<SubscriptionResponse>> listSubscriptions(String orgId, String spaceId) async
    test('test listSubscriptions', () async {
      // TODO
    });

    // `POST /api/orgs/{org_id}/catalog` — publish a resource to the org catalog.
    //
    //Future<CatalogItemResponse> publishCatalogItem(String orgId, PublishCatalogItemRequest publishCatalogItemRequest) async
    test('test publishCatalogItem', () async {
      // TODO
    });
  });
}
