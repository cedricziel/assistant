import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../features/connection/connection_provider.dart';
import 'models/server_capabilities.dart';

/// Fetches and caches server capability flags.
///
/// Falls back to [ServerCapabilities.disabled] when no client is available or
/// the request fails.  Uses a [CancelToken] so the in-flight request is
/// cancelled when the provider is disposed (prevents timer leaks in tests).
final capabilitiesProvider = FutureProvider<ServerCapabilities>((ref) async {
  final api = ref.watch(apiClientProvider);
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
