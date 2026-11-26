// RED→GREEN: openspec/changes/generative-ui-a2ui PR 14b.4.
import 'package:assistant_app/api/capabilities_provider.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/spaces/space_provider.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:dio/dio.dart' show CancelToken;
import 'package:flutter_test/flutter_test.dart';

/// Answers `link_hosts` per the space the caller has selected.
class _PerSpaceClient extends ApiClient {
  _PerSpaceClient(this._selected)
    : super(baseUrl: 'http://fake.test', token: 'test-token');

  final String? Function() _selected;

  @override
  Future<ServerCapabilities> getCapabilities({
    CancelToken? cancelToken,
  }) async => ServerCapabilities(
    voiceSend: false,
    voiceReceive: false,
    linkHosts: ['${_selected()}.example'],
  );
}

class _Selection extends SpaceSelectionNotifier {
  @override
  SpaceSelection build() => const SpaceSelection(spaceId: 'one');
}

void main() {
  test('reads ui.link_hosts from the capabilities', () async {
    final container = ProviderContainer(
      overrides: [
        capabilitiesProvider.overrideWith(
          (ref) async => const ServerCapabilities(
            voiceSend: false,
            voiceReceive: false,
            linkHosts: ['docs.example'],
          ),
        ),
      ],
    );
    addTearDown(container.dispose);

    await container.read(capabilitiesProvider.future);

    expect(container.read(uiLinkHostsProvider), ['docs.example']);
  });

  test('is empty while capabilities load', () {
    final container = ProviderContainer(
      overrides: [
        capabilitiesProvider.overrideWith((ref) => Future.any(const [])),
      ],
    );
    addTearDown(container.dispose);

    expect(container.read(uiLinkHostsProvider), isEmpty);
  });

  test('is empty when the capabilities call fails', () async {
    final container = ProviderContainer(
      retry: (_, _) => null,
      overrides: [
        capabilitiesProvider.overrideWith((ref) async => throw Exception('x')),
      ],
    );
    addTearDown(container.dispose);

    container.listen(uiLinkHostsProvider, (_, _) {});
    await container
        .read(capabilitiesProvider.future)
        .then<void>((_) {}, onError: (_) {});
    await Future<void>.delayed(Duration.zero);

    expect(container.read(uiLinkHostsProvider), isEmpty);
  });

  test('ServerCapabilities.disabled has no link hosts', () {
    expect(ServerCapabilities.disabled.linkHosts, isEmpty);
  });

  test('fromJson reads ui.link_hosts and tolerates its absence', () {
    expect(
      ServerCapabilities.fromJson({
        'ui': {
          'link_hosts': ['a.example'],
        },
      }).linkHosts,
      ['a.example'],
    );
    expect(ServerCapabilities.fromJson({}).linkHosts, isEmpty);
  });

  test('link hosts follow the selected space', () async {
    late ProviderContainer container;
    container = ProviderContainer(
      overrides: [
        spaceSelectionProvider.overrideWith(_Selection.new),
        apiClientProvider.overrideWith(
          (ref) => _PerSpaceClient(
            () => container.read(spaceSelectionProvider).spaceId,
          ),
        ),
      ],
    );
    addTearDown(container.dispose);
    container.listen(uiLinkHostsProvider, (_, _) {});

    await container.read(capabilitiesProvider.future);
    expect(container.read(uiLinkHostsProvider), ['one.example']);

    container
        .read(spaceSelectionProvider.notifier)
        .selectSpace(spaceId: 'two', spaceName: 'Two');
    await container.read(capabilitiesProvider.future);

    expect(container.read(uiLinkHostsProvider), [
      'two.example',
    ], reason: 'another org must not inherit the previous trusted hosts');
  });
}
