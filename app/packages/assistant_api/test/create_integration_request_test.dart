import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for CreateIntegrationRequest
void main() {
  final instance = CreateIntegrationRequestBuilder();
  // TODO add properties to the builder and call build()

  group(CreateIntegrationRequest, () {
    // Required for a custom URL and for a catalog entry whose auth kind is unknown (choose `none`, `header` or `oauth`). A `mixed` catalog entry may be narrowed to `header` or `oauth`.
    // IntegrationAuthKind authKind
    test('to test the property `authKind`', () async {
      // TODO
    });

    // Install this catalog entry. Its URL, auth kind and header template are copied at install time.
    // String catalogEntryId
    test('to test the property `catalogEntryId`', () async {
      // TODO
    });

    // Install a custom remote MCP server. Needs `name` and `auth_kind`.
    // String connectUrl
    test('to test the property `connectUrl`', () async {
      // TODO
    });

    // Defaults to `per_user` for OAuth and `shared` otherwise. User-layer integrations are always `per_user`.
    // CredentialMode credentialMode
    test('to test the property `credentialMode`', () async {
      // TODO
    });

    // Defaults to `true`.
    // bool enabled
    test('to test the property `enabled`', () async {
      // TODO
    });

    // Header that carries the credential. Required for `header` auth on a custom URL or an unknown catalog entry; rejected when the catalog entry supplies its own template.
    // String headerName
    test('to test the property `headerName`', () async {
      // TODO
    });

    // Text before the secret in the header value. Same rules as `header_name`.
    // String headerValuePrefix
    test('to test the property `headerValuePrefix`', () async {
      // TODO
    });

    // IntegrationLayer layer
    test('to test the property `layer`', () async {
      // TODO
    });

    // Defaults to the catalog entry's name. Required for a custom URL.
    // String name
    test('to test the property `name`', () async {
      // TODO
    });

    // Kebab-case, 1–32 characters. Defaults to the catalog entry's slug.
    // String slug
    test('to test the property `slug`', () async {
      // TODO
    });

    // Required for the space layer, rejected otherwise.
    // String spaceId
    test('to test the property `spaceId`', () async {
      // TODO
    });

    // Defaults to `confirm`.
    // IntegrationTrust trust
    test('to test the property `trust`', () async {
      // TODO
    });
  });
}
