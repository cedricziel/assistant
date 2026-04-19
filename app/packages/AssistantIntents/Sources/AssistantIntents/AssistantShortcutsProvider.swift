import AppIntents

/// Registers App Shortcuts so Siri and the Shortcuts app can discover
/// the assistant's intents without the user manually creating a shortcut.
public struct AssistantShortcutsProvider: AppShortcutsProvider {
    public static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: AskAssistantIntent(),
            phrases: [
                "Ask \(.applicationName) a question",
                "Ask \(.applicationName) something",
            ],
            shortTitle: "Ask Assistant",
            systemImageName: "bubble.left.fill"
        )
        AppShortcut(
            intent: RunWorkflowIntent(),
            phrases: [
                "Run \(.applicationName) workflow",
            ],
            shortTitle: "Run Workflow",
            systemImageName: "gearshape.2.fill"
        )
        AppShortcut(
            intent: ListPersonasIntent(),
            phrases: [
                "Show my \(.applicationName) personas",
            ],
            shortTitle: "List Personas",
            systemImageName: "person.2.fill"
        )
        AppShortcut(
            intent: ListWorkflowsIntent(),
            phrases: [
                "Show my \(.applicationName) workflows",
            ],
            shortTitle: "List Workflows",
            systemImageName: "list.bullet"
        )
        AppShortcut(
            intent: ListConversationsIntent(),
            phrases: [
                "Show my \(.applicationName) conversations",
            ],
            shortTitle: "List Conversations",
            systemImageName: "text.bubble.fill"
        )
    }
}
