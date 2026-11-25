import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for IntegrationDetail
void main() {
  final instance = IntegrationDetailBuilder();
  // TODO add properties to the builder and call build()

  group(IntegrationDetail, () {
    // IntegrationAuthKind authKind
    test('to test the property `authKind`', () async {
      // TODO
    });

    // String catalogEntryId
    test('to test the property `catalogEntryId`', () async {
      // TODO
    });

    // String connectUrl
    test('to test the property `connectUrl`', () async {
      // TODO
    });

    // DateTime createdAt
    test('to test the property `createdAt`', () async {
      // TODO
    });

    // CredentialMode credentialMode
    test('to test the property `credentialMode`', () async {
      // TODO
    });

    // bool enabled
    test('to test the property `enabled`', () async {
      // TODO
    });

    // bool hasCredential
    test('to test the property `hasCredential`', () async {
      // TODO
    });

    // Header that carries the credential, for `header` and `mixed` auth.
    // String headerName
    test('to test the property `headerName`', () async {
      // TODO
    });

    // Text before the secret in the header value, e.g. `Bearer `.
    // String headerValuePrefix
    test('to test the property `headerValuePrefix`', () async {
      // TODO
    });

    // `int_<uuid>`.
    // String id
    test('to test the property `id`', () async {
      // TODO
    });

    // IntegrationLayer layer
    test('to test the property `layer`', () async {
      // TODO
    });

    // String name
    test('to test the property `name`', () async {
      // TODO
    });

    // String ownerUserId
    test('to test the property `ownerUserId`', () async {
      // TODO
    });

    // String slug
    test('to test the property `slug`', () async {
      // TODO
    });

    // Space rows of an org-layer integration, limited to the spaces the caller belongs to unless the caller is an org admin.
    // BuiltList<SpaceAccessEntry> spaceAccess
    test('to test the property `spaceAccess`', () async {
      // TODO
    });

    // String spaceId
    test('to test the property `spaceId`', () async {
      // TODO
    });

    // IntegrationStatus status
    test('to test the property `status`', () async {
      // TODO
    });

    // IntegrationTrust trust
    test('to test the property `trust`', () async {
      // TODO
    });

    // DateTime updatedAt
    test('to test the property `updatedAt`', () async {
      // TODO
    });
  });
}
