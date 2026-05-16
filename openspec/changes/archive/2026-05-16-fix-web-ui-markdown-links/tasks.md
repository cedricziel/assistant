## Tasks

### Phase 1 — Failing tests

- [ ] Add `app/test/unit/features/chat/markdown_link_handler_test.dart` covering: - allowed schemes (`http`, `https`, `mailto`) trigger the injected launcher exactly once with `LaunchMode.externalApplication` - blocked schemes (`javascript:`, `data:`, `file:`, `assistant:`) do not call the launcher - launcher returning `false` surfaces a `"Could not open link"` snackbar - launcher throwing is caught, snackbar shown, error logged via `debugPrint`
- [ ] Add `app/test/widget/chat/chat_message_link_test.dart` that pumps a chat bubble containing `[Example](https://example.com)` and verifies a tap routes through the injected handler exactly once.
- [ ] Run `flutter test` and confirm RED (handler does not exist yet).

### Phase 2 — Implementation

- [ ] Create `app/lib/features/chat/markdown_link_handler.dart` exposing `MarkdownLinkHandler` with an injectable `UrlLauncher` typedef and a `defaultUrlLauncher` that forwards to `url_launcher.launchUrl`.
- [ ] Implement the scheme allow-list (`http`, `https`, `mailto`) and snackbar fallback inside the handler.
- [ ] Wire `onTapLink` on both `SmoothMarkdown` blocks in `app/lib/features/chat/chat_screen.dart` to delegate to the handler.
- [ ] Run `flutter test` and confirm GREEN.

### Phase 3 — Verification

- [ ] Manual check on web: open a chat, send `Visit [example](https://example.com)`, tap the link, confirm new tab opens.
- [ ] Manual check on macOS: same flow, confirm the system browser opens.
- [ ] `make lint-flutter && make test-flutter` (Flutter lints + tests).
- [ ] No Playwright baseline changes expected (behavioural-only); confirm baselines still match.
