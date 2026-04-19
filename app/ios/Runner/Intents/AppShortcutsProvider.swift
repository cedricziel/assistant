import AppIntents

/// Registers App Shortcuts so Siri and the Shortcuts app can discover
/// the assistant's intents without the user manually creating a shortcut.
@available(iOS 16.0, *)
struct AssistantShortcuts: AppShortcutsProvider {
    static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: AskAssistantIntent(),
            phrases: [
                "Ask \(.applicationName) a question",
                "Ask \(.applicationName) something",
            ],
            shortTitle: "Ask Assistant",
            systemImageName: "bubble.left.fill"
        )
    }
}
