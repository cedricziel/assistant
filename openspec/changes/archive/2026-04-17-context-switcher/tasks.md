## 1. Dependencies & Project Setup

- [x] 1.1 Add `shared_preferences` and `flutter_secure_storage` to `app/pubspec.yaml` (if not already present)
- [x] 1.2 Add `flutter_secure_storage` macOS entitlement group reusing the existing keychain group from #414
- [x] 1.3 Run `flutter pub get` and confirm `flutter analyze` is clean

## 2. Context Domain Model

- [x] 2.1 Create `app/lib/features/contexts/models/context_model.dart` with `AssistantContext` dataclass (`id`, `name`, `serverUrl`, `authToken?`, `createdAt`)
- [x] 2.2 Add `copyWith`, `toJson`, `fromJson` to `AssistantContext`
- [x] 2.3 Write unit tests for `AssistantContext` serialisation round-trip in `app/test/unit/contexts/context_model_test.dart`

## 3. Context Repository

- [x] 3.1 Create `app/lib/features/contexts/data/context_repository.dart` with `ContextRepository` class (CRUD + list)
- [x] 3.2 Implement `saveContext` — persist name/URL/id/createdAt to `SharedPreferences`, token to `FlutterSecureStorage`
- [x] 3.3 Implement `loadContexts` — read from `SharedPreferences`, sorted by `createdAt` ascending
- [x] 3.4 Implement `deleteContext` — remove from both stores by id
- [x] 3.5 Implement `getActiveContextId` / `setActiveContextId` in `SharedPreferences`
- [x] 3.6 Write unit tests for all repository operations using fake `SharedPreferences` in `app/test/unit/contexts/context_repository_test.dart`

## 4. Riverpod Providers

- [x] 4.1 Create `app/lib/features/contexts/providers/context_providers.dart` with `contextsProvider` (`AsyncNotifier<List<AssistantContext>>`)
- [x] 4.2 Create `activeContextProvider` (`AsyncNotifier<AssistantContext?>`) that loads/persists the active context ID via the repository
- [x] 4.3 Update `serverProfileProvider` (in `features/connection/`) to derive from `activeContextProvider` with fallback to `null`
- [x] 4.4 Write unit tests for `activeContextProvider` state transitions (activate, deactivate, delete active) in `app/test/unit/contexts/active_context_provider_test.dart`

## 5. Context Switcher Screen

- [x] 5.1 Create `app/lib/features/contexts/screens/context_switcher_screen.dart` with `ContextSwitcherScreen` widget
- [x] 5.2 Implement empty-state view ("No contexts yet. Tap + to add one.")
- [x] 5.4 Implement `ListView` of `ContextTile` widgets showing name, URL, and active indicator
- [x] 5.5 Implement FAB that opens a `CreateContextDialog` (name field, URL field, optional auth token field)
- [x] 5.6 Implement `CreateContextDialog` with validation (non-empty name, unique name, valid URL)
- [x] 5.7 Implement context activation on tile tap (set active, navigate to `/chat`)
- [x] 5.8 Implement long-press or swipe-to-delete on context tile with confirmation dialog
- [x] 5.9 Write widget tests for `ContextSwitcherScreen` in `app/test/widget/contexts/context_switcher_screen_test.dart`:
  - Empty state renders correctly
  - List shows contexts from provider
  - Tapping a context activates it and navigates
  - FAB opens dialog
  - Dialog validation errors shown
  - Delete removes context from list
  - Active context tile shows indicator

## 6. Context Edit Flow

- [x] 6.1 Create `app/lib/features/contexts/screens/edit_context_screen.dart` (or dialog) pre-filled with existing values
- [x] 6.2 Wire edit action from context tile (e.g., trailing edit icon)
- [x] 6.3 Write widget tests for edit flow in `app/test/widget/contexts/edit_context_test.dart`

## 7. Router Integration

- [x] 7.1 Add `/contexts` route to `app/lib/router/app_router.dart` pointing to `ContextSwitcherScreen`
- [x] 7.2 Add redirect logic: if `activeContextProvider` is `null` and current route is not `/contexts`, redirect to `/contexts`
- [x] 7.3 Guard redirect to prevent loop when already on `/contexts`
- [x] 7.4 Add "Contexts" entry to the navigation rail with appropriate icon
- [x] 7.5 Write widget tests for router redirect behavior in `app/test/widget/router/context_redirect_test.dart`

## 8. macOS Tray Integration

- [x] 8.1 Update tray menu builder to read `activeContextProvider` and display active context name as menu header
- [x] 8.2 Add "No active context" fallback text when `activeContextProvider` is `null`
- [x] 8.3 Add "Switch Context" submenu populated from `contextsProvider`
- [x] 8.4 Implement tray submenu item tap handler that calls `activeContextProvider.activate(context)`
- [x] 8.5 Write widget/integration tests for tray menu content in `app/test/widget/tray/tray_context_menu_test.dart`

## 9. Final Validation

- [x] 9.1 Run `flutter analyze --fatal-infos` — zero issues
- [x] 9.2 Run `flutter test` — all tests green
- [ ] 9.3 Smoke-test on macOS: create "Work" + "Personal" contexts, switch between them, verify tray menu updates
- [ ] 9.4 Smoke-test on web: create contexts, switch, confirm redirect on delete of active context
- [ ] 9.5 Confirm context switcher is fully navigable by macOS VoiceOver
