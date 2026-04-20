import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Streams real-time connectivity status changes from the OS.
///
/// Emits a list of active connection types (wifi, mobile, ethernet, etc.)
/// or an empty list / [ConnectivityResult.none] when offline.
final connectivityProvider =
    StreamProvider.autoDispose<List<ConnectivityResult>>((ref) {
      return Connectivity().onConnectivityChanged;
    });

/// Derived provider — `true` when at least one network transport is available.
final isOnlineProvider = Provider.autoDispose<bool>((ref) {
  final connectivity = ref.watch(connectivityProvider);
  return connectivity.when(
    data: (results) =>
        results.isNotEmpty &&
        !results.every((r) => r == ConnectivityResult.none),
    loading: () => true, // Assume online until proven otherwise.
    error: (_, _) => true, // Assume online on error (fail open).
  );
});
