## Why

Hyperlinks rendered from markdown in the chat (`[text](https://example.com)`) do not open when tapped. The `SmoothMarkdown` widget in `app/lib/features/chat/chat_screen.dart` supports an `onTapLink` callback but the chat screen does not wire one up, so anchor taps are silently swallowed. Users see hyperlinks, taps register, and nothing happens — they have to copy the URL out of the bubble manually.

## What Changes

- Wire an `onTapLink` handler into every `SmoothMarkdown` usage in the chat screen (streaming and historical messages).
- Use `url_launcher` (already a dependency) to open the URL in an external application on native platforms and in a new tab on web.
- Reject obviously unsafe schemes (only allow `http`, `https`, and `mailto`).
- Show a snackbar / toast when the URL cannot be opened, so failures are visible rather than silent.

## Non-goals

- Custom in-app web view — links open in the platform browser.
- Rewriting the markdown stack (we keep `flutter_smooth_markdown`).
- Handling links in other markdown surfaces (skills, logs) — those can adopt the same shared helper later but are out of scope for this fix.
- Deep-link interception (e.g. `assistant://...`) — covered by other proposals.

## Capabilities

### Modified Capabilities

- `chat-markdown-rendering` (new spec) — anchor taps in chat markdown open the target URL through the platform browser.

## Impact

- `app/lib/features/chat/chat_screen.dart` — pass `onTapLink` to both `SmoothMarkdown` blocks.
- New helper `app/lib/features/chat/markdown_link_handler.dart` so the tap callback is testable in isolation and reusable.
- Widget test for the chat bubble verifying that a tap on a markdown anchor invokes the handler with the expected URL.
- No backend changes. No spec changes outside the new `chat-markdown-rendering` capability.

## Visual / UI change

Behavioural only. No new pixels — anchor styling is unchanged. Playwright screenshot baselines unaffected.

## User-facing documentation

Not required. Behaviour matches the user's expectation of a markdown renderer; no separate docs page warranted.
