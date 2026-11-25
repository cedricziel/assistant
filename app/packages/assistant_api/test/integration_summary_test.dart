import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for IntegrationSummary
void main() {
  final instance = IntegrationSummaryBuilder();
  // TODO add properties to the builder and call build()

  group(IntegrationSummary, () {
    // IntegrationAuthKind authKind
    test('to test the property `authKind`', () async {
      // TODO
    });

    // The catalog entry it was installed from; absent for a custom URL.
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

    // Whether a credential is stored for the caller's subject: the shared credential, or the caller's own for a per-user integration. The value itself is never returned.
    // bool hasCredential
    test('to test the property `hasCredential`', () async {
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

    // The owner of a user-layer integration.
    // String ownerUserId
    test('to test the property `ownerUserId`', () async {
      // TODO
    });

    // Unique within its layer and owner. Part of its tool names.
    // String slug
    test('to test the property `slug`', () async {
      // TODO
    });

    // The space of a space-layer integration.
    // String spaceId
    test('to test the property `spaceId`', () async {
      // TODO
    });

    // Whether the caller can use it. For an org admin looking at another user's integration, the status is the owner's.
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
