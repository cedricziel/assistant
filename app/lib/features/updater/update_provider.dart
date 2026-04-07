import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'update_service.dart';

/// Holds an [UpdateInfo] when a newer version is available, [null] otherwise.
class UpdateCheckNotifier extends AsyncNotifier<UpdateInfo?> {
  @override
  Future<UpdateInfo?> build() => UpdateService().checkForUpdate();

  /// Dismiss the update notification for this session.
  void dismiss() => state = const AsyncData(null);
}

final updateCheckProvider =
    AsyncNotifierProvider<UpdateCheckNotifier, UpdateInfo?>(
  UpdateCheckNotifier.new,
);
