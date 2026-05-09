import 'package:web/web.dart' as web;

import 'space_selection_storage.dart';

/// Returns a [SpaceSelectionStorage] backed by `window.localStorage`. Only
/// loaded on web via conditional import from
/// `space_selection_storage.dart`.
SpaceSelectionStorage makeWebStorage() => _LocalStorageSpaceSelectionStorage();

class _LocalStorageSpaceSelectionStorage implements SpaceSelectionStorage {
  @override
  String? read(String key) {
    try {
      return web.window.localStorage.getItem(key);
    } catch (_) {
      return null;
    }
  }

  @override
  void write(String key, String value) {
    try {
      web.window.localStorage.setItem(key, value);
    } catch (_) {
      // Quota or private-mode failures are non-fatal.
    }
  }

  @override
  void remove(String key) {
    try {
      web.window.localStorage.removeItem(key);
    } catch (_) {}
  }
}
