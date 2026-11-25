import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/integrations/caller_roles.dart';
import 'package:assistant_app/features/integrations/install_integration_view_model.dart';
import 'package:built_collection/built_collection.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'integration_fakes.dart';

CatalogEntrySummary _entry({
  String id = 'curated/linear-app-mcp',
  String name = 'Linear',
  CatalogAuthKindResponse kind = CatalogAuthKindResponse.oauth,
  String? headerName,
  List<IntegrationLayer> layers = const [
    IntegrationLayer.org,
    IntegrationLayer.space,
    IntegrationLayer.user,
  ],
}) {
  return CatalogEntrySummary(
    (b) => b
      ..id = id
      ..name = name
      ..description = 'About'
      ..domain = 'linear.app'
      ..connectUrl = 'https://mcp.linear.app/mcp'
      ..categories = ListBuilder<String>()
      ..installableLayers = ListBuilder<IntegrationLayer>(layers)
      ..auth = CatalogAuthResponse(
        (a) => a
          ..kind = kind
          ..headerName = headerName,
      ).toBuilder(),
  );
}

InstallArgs _args(
  CatalogEntrySummary? entry, {
  List<IntegrationLayer> layers = const [
    IntegrationLayer.org,
    IntegrationLayer.space,
    IntegrationLayer.user,
  ],
  List<SpaceSummary>? spaces,
}) => InstallArgs(
  entry: entry,
  layers: layers,
  spaces: spaces ?? [spaceSummary('eng'), spaceSummary('ops')],
);

({ProviderContainer container, FakeIntegrationsApi api, InstallArgs args})
_setup(InstallArgs args) {
  final api = FakeIntegrationsApi();
  final container = ProviderContainer(
    overrides: [
      apiClientProvider.overrideWithValue(FakeApiClient(integrationsApi: api)),
    ],
  );
  addTearDown(container.dispose);
  container.listen(installIntegrationViewModelProvider(args), (_, _) {});
  return (container: container, api: api, args: args);
}

InstallIntegrationViewModel _vm(
  ({ProviderContainer container, FakeIntegrationsApi api, InstallArgs args}) s,
) => s.container.read(installIntegrationViewModelProvider(s.args).notifier);

InstallState _state(
  ({ProviderContainer container, FakeIntegrationsApi api, InstallArgs args}) s,
) => s.container.read(installIntegrationViewModelProvider(s.args));

void main() {
  group('defaults', () {
    test('prefills slug and name from the catalog entry', () {
      final s = _setup(_args(_entry()));

      expect(_state(s).slug, 'linear-app-mcp');
      expect(_state(s).name, 'Linear');
      expect(_state(s).trust, IntegrationTrust.confirm);
    });

    test('shortens and cleans a long or odd catalog id into a slug', () {
      final s = _setup(
        _args(
          _entry(id: 'curated/Some_Very.Long Name-for-a-catalog-entry-mcp'),
        ),
      );

      expect(_state(s).slug, 'some-very-long-name-for-a-catalo');
      expect(_state(s).slugError, isNull);
    });

    test('starts at the least privileged allowed layer', () {
      expect(_state(_setup(_args(_entry()))).layer, IntegrationLayer.user);
      expect(
        _state(_setup(_args(_entry(), layers: [IntegrationLayer.org]))).layer,
        IntegrationLayer.org,
      );
      expect(
        _state(
          _setup(
            _args(
              _entry(),
              layers: [IntegrationLayer.org, IntegrationLayer.space],
            ),
          ),
        ).layer,
        IntegrationLayer.space,
      );
    });

    test('the space layer starts with the first administered space', () {
      final s = _setup(_args(_entry(), layers: [IntegrationLayer.space]));

      expect(_state(s).spaceId, 'eng');
    });

    test(
      'credential mode defaults to per_user for OAuth, shared otherwise',
      () {
        expect(
          _state(_setup(_args(_entry(), layers: [IntegrationLayer.org])))
              .credentialMode,
          CredentialMode.perUser,
        );
        expect(
          _state(
            _setup(
              _args(
                _entry(
                  kind: CatalogAuthKindResponse.header,
                  headerName: 'X-Key',
                ),
                layers: [IntegrationLayer.org],
              ),
            ),
          ).credentialMode,
          CredentialMode.shared,
        );
      },
    );
  });

  group('slug validation', () {
    for (final (slug, valid) in [
      ('linear', true),
      ('a-b-2', true),
      ('x', true),
      ('a' * 32, true),
      ('', false),
      ('Linear', false),
      ('a--b', false),
      ('-a', false),
      ('a-', false),
      ('a_b', false),
      ('a b', false),
      ('a' * 33, false),
    ]) {
      test('"$slug" is ${valid ? 'valid' : 'invalid'}', () {
        final s = _setup(_args(_entry()));
        _vm(s).setSlug(slug);

        expect(_state(s).slugError == null, valid);
      });
    }
  });

  group('layers', () {
    test('a layer outside the allowed ones is ignored', () {
      final s = _setup(
        _args(_entry(), layers: [IntegrationLayer.user, IntegrationLayer.org]),
      );

      _vm(s).setLayer(IntegrationLayer.space);

      expect(_state(s).layer, IntegrationLayer.user);
    });

    test('the credential mode only applies to org and space', () {
      final s = _setup(_args(_entry()));

      expect(_state(s).showsCredentialMode, isFalse);
      _vm(s).setLayer(IntegrationLayer.org);
      expect(_state(s).showsCredentialMode, isTrue);
      _vm(s).setLayer(IntegrationLayer.space);
      expect(_state(s).showsCredentialMode, isTrue);
    });

    test('unknown auth needs a choice before it can be submitted', () {
      final s = _setup(
        _args(_entry(kind: CatalogAuthKindResponse.unknownDefaultOpenApi)),
      );

      expect(_state(s).needsAuthChoice, isTrue);
      expect(_state(s).canSubmit, isFalse);

      _vm(s).setAuthKind(IntegrationAuthKind.oauth);

      expect(_state(s).canSubmit, isTrue);
    });

    test('a known auth kind needs no choice', () {
      final s = _setup(_args(_entry()));

      expect(_state(s).needsAuthChoice, isFalse);
      expect(_state(s).canSubmit, isTrue);
    });

    test('header auth without a catalog header asks for a header name', () {
      final s = _setup(
        _args(_entry(kind: CatalogAuthKindResponse.unknownDefaultOpenApi)),
      );
      _vm(s).setAuthKind(IntegrationAuthKind.header);

      expect(_state(s).needsHeaderName, isTrue);
      expect(_state(s).canSubmit, isFalse);

      _vm(s).setHeaderName('X-Api-Key');

      expect(_state(s).canSubmit, isTrue);
    });
  });

  group('submit', () {
    test('installs a catalog entry at the chosen layer', () async {
      final s = _setup(_args(_entry()));
      CreateIntegrationRequest? sent;
      s.api.onCreate = (request) async {
        sent = request;
        return integrationDetail(
          'int_new',
          status: IntegrationStatus.needsConnection,
        );
      };
      _vm(s)
        ..setLayer(IntegrationLayer.org)
        ..setSlug('linear-eng')
        ..setName('Linear (eng)')
        ..setCredentialMode(CredentialMode.shared)
        ..setTrust(IntegrationTrust.trust);

      await _vm(s).submit();

      expect(sent!.layer, IntegrationLayer.org);
      expect(sent!.catalogEntryId, 'curated/linear-app-mcp');
      expect(sent!.slug, 'linear-eng');
      expect(sent!.name, 'Linear (eng)');
      expect(sent!.credentialMode, CredentialMode.shared);
      expect(sent!.trust, IntegrationTrust.trust);
      expect(sent!.connectUrl, isNull);
      expect(sent!.authKind, isNull);
      expect(sent!.spaceId, isNull);
      expect(_state(s).created!.id, 'int_new');
      expect(_state(s).submitting, isFalse);
      expect(_state(s).error, isNull);
    });

    test('a space install names the space', () async {
      final s = _setup(_args(_entry()));
      CreateIntegrationRequest? sent;
      s.api.onCreate = (request) async {
        sent = request;
        return integrationDetail('int_new');
      };
      _vm(s)
        ..setLayer(IntegrationLayer.space)
        ..setSpace('ops');

      await _vm(s).submit();

      expect(sent!.layer, IntegrationLayer.space);
      expect(sent!.spaceId, 'ops');
    });

    test('a user install leaves out space and credential mode', () async {
      final s = _setup(_args(_entry()));
      CreateIntegrationRequest? sent;
      s.api.onCreate = (request) async {
        sent = request;
        return integrationDetail('int_new');
      };

      await _vm(s).submit();

      expect(sent!.layer, IntegrationLayer.user);
      expect(sent!.spaceId, isNull);
      expect(sent!.credentialMode, isNull);
    });

    test(
      'an unknown-auth entry sends the chosen auth kind and header',
      () async {
        final s = _setup(
          _args(_entry(kind: CatalogAuthKindResponse.unknownDefaultOpenApi)),
        );
        CreateIntegrationRequest? sent;
        s.api.onCreate = (request) async {
          sent = request;
          return integrationDetail('int_new');
        };
        _vm(s)
          ..setAuthKind(IntegrationAuthKind.header)
          ..setHeaderName('X-Api-Key');

        await _vm(s).submit();

        expect(sent!.authKind, IntegrationAuthKind.header);
        expect(sent!.headerName, 'X-Api-Key');
      },
    );

    test('a custom URL installs without a catalog entry', () async {
      final s = _setup(_args(null));
      CreateIntegrationRequest? sent;
      s.api.onCreate = (request) async {
        sent = request;
        return integrationDetail('int_new');
      };

      expect(_state(s).canSubmit, isFalse, reason: 'needs url, slug, name');
      _vm(s)
        ..setConnectUrl('https://mcp.example.com/mcp')
        ..setSlug('example')
        ..setName('Example')
        ..setAuthKind(IntegrationAuthKind.none);
      expect(_state(s).canSubmit, isTrue);

      await _vm(s).submit();

      expect(sent!.catalogEntryId, isNull);
      expect(sent!.connectUrl, 'https://mcp.example.com/mcp');
      expect(sent!.authKind, IntegrationAuthKind.none);
    });

    test('rejects a custom URL that is not http(s)', () {
      final s = _setup(_args(null));
      _vm(s).setConnectUrl('ftp://nope');

      expect(_state(s).urlError, isNotNull);
      _vm(s).setConnectUrl('not a url');
      expect(_state(s).urlError, isNotNull);
      _vm(s).setConnectUrl('http://localhost:3000/mcp');
      expect(_state(s).urlError, isNull);
    });

    test('an invalid form is not submitted', () async {
      final s = _setup(_args(_entry()));
      _vm(s).setSlug('Not Kebab');

      await _vm(s).submit();

      expect(s.api.calls, isEmpty);
    });

    test('403 explains the policy', () async {
      final s = _setup(_args(_entry()));
      s.api.onCreate = (request) async => throw dioError(403);

      await _vm(s).submit();

      expect(_state(s).error, contains('does not allow'));
      expect(_state(s).created, isNull);
      expect(_state(s).submitting, isFalse);
    });

    test('409 flags the slug as taken until it changes', () async {
      final s = _setup(_args(_entry()));
      s.api.onCreate = (request) async => throw dioError(409);

      await _vm(s).submit();

      expect(_state(s).slugError, contains('already'));
      expect(_state(s).canSubmit, isFalse);

      _vm(s).setSlug('linear-2');

      expect(_state(s).slugError, isNull);
      expect(_state(s).canSubmit, isTrue);
    });

    test('422 shows what the server rejected', () async {
      final s = _setup(_args(_entry()));
      s.api.onCreate = (request) async =>
          throw dioError(422, body: {'error': 'connect_url must be https'});

      await _vm(s).submit();

      expect(_state(s).error, contains('connect_url must be https'));
    });

    test('submitting is true while the request is in flight', () async {
      final s = _setup(_args(_entry()));
      s.api.onCreate = (request) => Future.delayed(
        const Duration(milliseconds: 10),
        () => integrationDetail('int_new'),
      );

      final pending = _vm(s).submit();
      expect(_state(s).submitting, isTrue);
      await pending;

      expect(_state(s).submitting, isFalse);
    });
  });

  group('what may be installed', () {
    CallerRoles roles({
      bool orgAdmin = false,
      List<String> adminOf = const [],
    }) => CallerRoles(
      spaces: [spaceSummary('eng'), spaceSummary('ops')],
      roles: {
        if (orgAdmin) 'eng': 'org_admin',
        for (final id in adminOf) id: 'space_admin',
      },
    );

    test('a catalog entry offers its installable layers', () {
      final args = installArgsForEntry(
        _entry(layers: [IntegrationLayer.user]),
        roles(),
      );

      expect(args!.layers, [IntegrationLayer.user]);
    });

    test('the space layer is dropped when the caller administers no space', () {
      final args = installArgsForEntry(
        _entry(layers: [IntegrationLayer.space, IntegrationLayer.user]),
        roles(),
      );

      expect(args!.layers, [IntegrationLayer.user]);
    });

    test('an entry installable nowhere yields no install', () {
      expect(installArgsForEntry(_entry(layers: const []), roles()), isNull);
    });

    test('custom URLs are open to org admins and, by policy, to others', () {
      expect(installArgsForCustom(policy(), roles(orgAdmin: true))!.layers, [
        IntegrationLayer.org,
      ]);
      expect(
        installArgsForCustom(
          policy(customUrls: true),
          roles(adminOf: ['eng']),
        )!.layers,
        [IntegrationLayer.space, IntegrationLayer.user],
      );
      expect(
        installArgsForCustom(
          policy(customUrls: true, userIntegrations: false),
          roles(),
        ),
        isNull,
      );
      expect(installArgsForCustom(policy(), roles()), isNull);
    });
  });
}
