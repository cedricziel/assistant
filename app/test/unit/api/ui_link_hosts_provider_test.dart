// RED→GREEN: openspec/changes/generative-ui-a2ui PR 14b.4.
import 'package:assistant_app/api/capabilities_provider.dart';
import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

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
}
