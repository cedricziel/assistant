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
