import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'space_selection_storage_stub.dart'
    if (dart.library.js_interop) 'space_selection_storage_web.dart'
    as platform;

/// Minimal key-value-store interface for [SpaceSelectionNotifier]'s
/// web-only persistence layer.
///
/// Production: `space_selection_storage_web.dart` on web, the stub
/// (in-memory no-op) everywhere else. Tests inject an in-memory fake via
/// [spaceSelectionStorageProvider].
abstract class SpaceSelectionStorage {
  String? read(String key);
  void write(String key, String value);
  void remove(String key);
}

/// Provider for the platform-appropriate [SpaceSelectionStorage]. Tests
/// override this with an in-memory fake.
final spaceSelectionStorageProvider = Provider<SpaceSelectionStorage>((ref) {
  if (kIsWeb) return platform.makeWebStorage();
  return _NoopSpaceSelectionStorage();
});

/// Used on native platforms. The selection stays in-memory — same behavior
/// as before this change.
class _NoopSpaceSelectionStorage implements SpaceSelectionStorage {
  @override
  String? read(String key) => null;
  @override
  void write(String key, String value) {}
  @override
  void remove(String key) {}
}
