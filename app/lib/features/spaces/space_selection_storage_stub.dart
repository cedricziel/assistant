import 'space_selection_storage.dart';

/// Stub used on non-web platforms. The real web implementation lives in
/// `space_selection_storage_web.dart` and is selected by conditional import.
SpaceSelectionStorage makeWebStorage() {
  // Should never be called: the runtime check in
  // `spaceSelectionStorageProvider` only selects this branch on native, and
  // returns a no-op directly. This stub exists only to satisfy the
  // conditional-import contract on the VM build.
  throw UnsupportedError(
    'space_selection_storage_web.dart is only available on web',
  );
}
