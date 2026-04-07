---
name: ux-principles
description: >
  Flutter UX/UI principles for the assistant app. Covers dark-theme colour
  tokens, responsive breakpoints (Flutter adaptive layouts), accessibility,
  Riverpod state patterns for loading/error/empty states, and platform
  conventions for web and macOS. Use when building or reviewing any Flutter
  screen or widget in app/.
license: MIT
---

# UX Principles (Flutter)

The frontend is a Flutter 3.x application (`app/`) targeting web and macOS.
State is managed with Riverpod 3.x. Navigation uses `go_router 17.x`.
The Rust backend serves the compiled Flutter web app embedded in the binary.

---

## Dark Theme Colour Tokens

The app uses a consistent dark palette. Apply these values via a `ThemeData`
extension or a shared `AppColors` class — never hard-code hex literals in
widgets.

| Token                | Value                  | Usage                         |
| -------------------- | ---------------------- | ----------------------------- |
| Background (body)    | `#020611`              | Scaffold background           |
| Surface              | `#0a1628`              | Cards, panels, dialogs        |
| Surface border       | `#0b1b32`              | Card/container borders        |
| Text primary         | `#e5e9f0`              | Headings, body text           |
| Text secondary       | `#8aa5d8`              | Labels, metadata, hints       |
| Accent blue          | `#6ec6ff`              | Links, focus rings            |
| Accent brand         | `#7aa2ff`              | Brand text, active indicators |
| Primary button       | `#2563eb`              | CTA buttons                   |
| Primary button hover | `#1d4ed8`              | Button hover/pressed state    |
| Error background     | `rgba(239,68,68,0.15)` | Error banners                 |
| Error border         | `rgba(239,68,68,0.3)`  | Error container borders       |
| Error text           | `#fca5a5`              | Error message text            |
| Success              | `#4ade80`              | Status badges, confirmations  |
| Warning              | `#fbbf24`              | Warning indicators            |

**Rules:**

- Never use `Colors.white` for text — use the `#e5e9f0` token.
- Never use `Colors.black` for backgrounds — use `#020611`.
- Maintain minimum 4.5:1 contrast ratio (WCAG AA) for all text.
- Use `Color.fromRGBO()` or `Color(0xFFrrggbb)` notation for custom tokens.

---

## Responsive Breakpoints

Three adaptive layout tiers, matching the Playwright viewport sizes:

| Tier    | Width      | Flutter pattern                             |
| ------- | ---------- | ------------------------------------------- |
| Desktop | >= 900 px  | `NavigationRail` (left) + wide content area |
| Tablet  | 640–899 px | `Drawer` (hamburger) + medium content       |
| Mobile  | < 640 px   | `NavigationBar` (bottom tabs) + full-width  |

Use `LayoutBuilder` or `MediaQuery.of(context).size.width` to switch layouts:

```dart
Widget build(BuildContext context) {
  final width = MediaQuery.of(context).size.width;
  if (width >= 900) return _DesktopLayout();
  if (width >= 640) return _TabletLayout();
  return _MobileLayout();
}
```

**Rules:**

- Touch targets: minimum 48×48 logical pixels (Material / iOS HIG).
- Never hardcode pixel widths in individual widgets — read from `MediaQuery`.
- Content areas should use `SingleChildScrollView` or `ListView` so they work at any height.

---

## Accessibility

- Use semantic widgets (`ElevatedButton`, `TextButton`) over raw `GestureDetector`.
- Provide `Semantics` labels for icon-only buttons and custom widgets.
- All interactive elements must have sufficient contrast (see tokens above).
- Focus traversal follows visual order — avoid manual `FocusNode` ordering unless required.
- Use `Tooltip` for icon-only actions.

---

## Loading / Error / Empty States

Every async screen must handle all three states. Pattern using Riverpod:

```dart
@override
Widget build(BuildContext context, WidgetRef ref) {
  final state = ref.watch(myProvider);
  return state.when(
    loading: () => const Center(child: CircularProgressIndicator()),
    error: (err, _) => Center(child: Text('Error: $err')),
    data: (items) => items.isEmpty
        ? const _EmptyState()
        : _ContentList(items: items),
  );
}
```

**Empty state widget rules:**

- Always provide a descriptive message explaining why the list is empty.
- Include a call-to-action (button/link) when an action can populate the list.
- Never show a blank screen or only a spinner on empty data.

---

## Form Patterns

- Use `TextFormField` inside a `Form` with a `GlobalKey<FormState>` for validation.
- Validate on `autovalidateMode: AutovalidateMode.onUserInteraction`.
- Show field-level errors via `validator` return value (not a separate overlay).
- Disable the submit button while an async operation is in flight.

```dart
ElevatedButton(
  onPressed: isLoading ? null : _submit,
  child: isLoading
      ? const SizedBox.square(dimension: 16, child: CircularProgressIndicator(strokeWidth: 2))
      : const Text('Save'),
)
```

---

## Platform Conventions

### Web

- The app is served embedded in the Rust binary at `/` (Flutter web build).
- Auth token is stored in `flutter_secure_storage` (falls back to `localStorage` on web).
- Deep-linking is handled by `go_router`; the Rust SPA handler serves `index.html` for any unmatched path.

### macOS

- Follows macOS HIG: menu bar, keyboard shortcuts (`⌘W`, `⌘R`), system font.
- Use `macos_ui` or native `CupertinoX` widgets where platform behaviour is expected (scroll physics, context menus).
- Sandbox entitlements in `macos/Runner/DebugProfile.entitlements` must include `com.apple.security.network.client` for API calls.
