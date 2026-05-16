## 1. Define the shortcut Intent and Action

- [x] 1.1 Create a `NewChatIntent` class extending `Intent` in a new file `app/lib/features/chat/new_chat_intent.dart`
- [x] 1.2 Create a `NewChatAction` class extending `Action<NewChatIntent>` that calls `ConversationListNotifier.createConversation()` and navigates to the new conversation via `go_router`. Include a boolean guard to prevent duplicate in-flight requests.

## 2. Register the shortcut globally in the router shell

- [x] 2.1 Wrap the `NavShell` child (in `app/lib/router/app_router.dart`) with a `Shortcuts` widget mapping `SingleActivator(LogicalKeyboardKey.keyN, meta: true)` and `SingleActivator(LogicalKeyboardKey.keyN, control: true)` to `NewChatIntent`
- [x] 2.2 Wrap with a corresponding `Actions` widget that binds `NewChatIntent` to `NewChatAction`, passing the required `WidgetRef` and `BuildContext` for provider access and navigation

## 3. Platform-aware modifier handling

- [x] 3.1 Use `defaultTargetPlatform` to register only Cmd+N on macOS/iOS and only Ctrl+N on other platforms, avoiding both firing or conflicting

## 4. Testing

- [x] 4.1 Write a widget test that simulates Cmd+N keypress and verifies a new conversation is created (mock the API) and navigation occurs
- [x] 4.2 Write a widget test that simulates two rapid Cmd+N presses and verifies only one conversation is created (debounce guard)
- [x] 4.3 Manually verify on macOS desktop and web that the shortcut works from the chat screen and from a non-chat screen (e.g., personas)
