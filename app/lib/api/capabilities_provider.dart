import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../features/connection/connection_provider.dart';
import '../features/spaces/space_provider.dart';
import 'models/server_capabilities.dart';

/// Fetches and caches server capability flags.
///
/// Falls back to [ServerCapabilities.disabled] when no client is available or
/// the request fails.  Uses a [CancelToken] so the in-flight request is
/// cancelled when the provider is disposed (prevents timer leaks in tests).
final capabilitiesProvider = FutureProvider<ServerCapabilities>((ref) async {
  final api = ref.watch(apiClientProvider);
  // The client reads the selected space per request, so a space change does
  // not rebuild it. Capabilities (including `ui.link_hosts`) are per org, so
  // refetch on every space change instead of keeping the previous org's.
  ref.watch(spaceSelectionProvider.select((s) => s.spaceId));
  if (api == null) return ServerCapabilities.disabled;
  final cancelToken = CancelToken();
  ref.onDispose(cancelToken.cancel);
  try {
    return await api.getCapabilities(cancelToken: cancelToken);
  } catch (_) {
    return ServerCapabilities.disabled;
  }
});

/// Hosts whose A2UI `openUrl` targets open without a confirmation prompt
/// (`ui.link_hosts`).
///
/// Empty while capabilities load and when the call fails or returns no list,
/// so the failure mode is more prompts, never fewer.
final uiLinkHostsProvider = Provider<List<String>>(
  (ref) => ref.watch(capabilitiesProvider).asData?.value.linkHosts ?? const [],
);
