import AppIntents

/// App Intent that returns recent conversations.
public struct ListConversationsIntent: AppIntent {
    public static var title: LocalizedStringResource = "List Conversations"
    public static var description = IntentDescription(
        "Get the list of recent assistant conversations."
    )

    @Parameter(
        title: "Limit",
        description: "Maximum number of conversations to return.",
        default: 20
    )
    public var limit: Int

    public init() {}

    public func perform() async throws -> some IntentResult & ReturnsValue<[ConversationEntity]> & ProvidesDialog {
        let client = AssistantAPIClient.shared

        do {
            let conversations = try await client.listConversations()
            let limited = Array(conversations.prefix(limit))
            let entities = limited.map {
                ConversationEntity(
                    id: $0.id,
                    title: $0.title ?? "Untitled",
                    updatedAt: $0.updatedAt
                )
            }
            return .result(
                value: entities,
                dialog: "Found \(entities.count) conversation\(entities.count == 1 ? "" : "s")."
            )
        } catch AssistantAPIClient.APIError.noCredentials {
            return .result(
                value: [],
                dialog: "Please open the app and connect to your assistant server first."
            )
        } catch {
            return .result(
                value: [],
                dialog: "I couldn't reach your assistant server."
            )
        }
    }
}
