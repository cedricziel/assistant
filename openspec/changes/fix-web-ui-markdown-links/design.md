## Context

The chat bubble renderer is in `chat_screen.dart` (around line 774). It instantiates two `SmoothMarkdown` widgets — one for the streaming message and one for finalised history rows — neither of which sets `onTapLink`. The `flutter_smooth_markdown` package (0.7.2 in `app/pubspec.yaml`) exposes `final void Function(String url)? onTapLink;` on both `SmoothMarkdown` and `StreamMarkdown` (verified in `.pub-cache/.../widgets/smooth_markdown.dart` and `widgets/stream_markdown.dart`).

`url_launcher: ^6.3.2` is already a dependency, so no new packages are needed.

## Goals / Non-Goals

**Goals**

- Tapping `[text](url)` opens the URL.
- One handler, used by both the streaming and finalised `SmoothMarkdown` blocks.
- The handler is unit-testable without a `WidgetTester` — pure function with an injectable launcher.
- Unsupported / dangerous schemes are rejected loudly (snackbar) rather than silently.

**Non-goals**

- In-app web view, custom URL schemes (`assistant://...`), or telemetry on link clicks.

## Decisions

### D1: Extract `MarkdownLinkHandler` helper

**Choice:** Add `app/lib/features/chat/markdown_link_handler.dart` exposing a small class:

```dart
class MarkdownLinkHandler {
  MarkdownLinkHandler({
    required this.context,
    UrlLauncher launcher = defaultUrlLauncher,
  }) : _launcher = launcher;

  final BuildContext context;
  final UrlLauncher _launcher;

  Future<void> onTap(String url) async { ... }
}

typedef UrlLauncher = Future<bool> Function(Uri uri, {LaunchMode mode});
```

Why a class and not a free function: the helper needs a `BuildContext` for the snackbar fallback. Injecting the launcher keeps the unit test free of plugin platform channels.

### D2: Scheme allow-list

**Choice:** Allow `http`, `https`, `mailto`. Everything else is rejected with a snackbar `"Cannot open link: <url>"`. We deliberately reject `javascript:`, `data:`, `file:`, and other schemes that have caused XSS in markdown ecosystems.

**Why:** The renderer is told the URL by the model, which means a prompt-injected response could plant a `javascript:` link. The allow-list is the cheapest mitigation. Unknown schemes that are legitimate (e.g. `slack://`) can be added later behind feature flags.

### D3: Launch mode

**Choice:** `LaunchMode.externalApplication` on every platform. On web the plugin already maps that to opening a new tab.

**Why:** Users expect external link clicks to leave the chat surface (or open a new tab). No in-app browser today, no plan to add one in this change.

## Risks / Trade-offs

- **Prompt-injected links to malicious sites:** the renderer cannot judge intent. The user still has to click. We accept the same risk that every chat surface accepts; the allow-list closes the loudest foot-guns.
- **Test coverage of the streaming branch:** the streaming bubble re-creates `SmoothMarkdown` repeatedly during stream ticks. We need one widget test that fires `tester.tap` on a rendered anchor and asserts the handler ran — that covers both bubbles because they share the helper.

## Migration Plan

None. Behavioural change only, no stored state.
