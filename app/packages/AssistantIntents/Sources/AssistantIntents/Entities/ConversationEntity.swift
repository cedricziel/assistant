import AppIntents

/// An App Entity representing an assistant conversation.
public struct ConversationEntity: AppEntity {
    public static var typeDisplayRepresentation = TypeDisplayRepresentation(name: "Conversation")
    public static var defaultQuery = ConversationEntityQuery()

    public var id: String
    public var title: String
    public var updatedAt: String?

    public var displayRepresentation: DisplayRepresentation {
        DisplayRepresentation(title: "\(title)")
    }

    public init(id: String, title: String, updatedAt: String? = nil) {
        self.id = id
        self.title = title
        self.updatedAt = updatedAt
    }
}

/// Query for conversation entities backed by the assistant REST API.
public struct ConversationEntityQuery: EntityQuery {
    public init() {}

    public func entities(for identifiers: [String]) async throws -> [ConversationEntity] {
        let all = try await suggestedEntities()
        return all.filter { identifiers.contains($0.id) }
    }

    public func suggestedEntities() async throws -> [ConversationEntity] {
        do {
            let conversations = try await AssistantAPIClient.shared.listConversations()
            return conversations.map {
                ConversationEntity(
                    id: $0.id,
                    title: $0.title ?? "Untitled",
                    updatedAt: $0.updatedAt
                )
            }
        } catch {
            return []
        }
    }
}

extension ConversationEntityQuery: EntityStringQuery {
    public func entities(matching string: String) async throws -> [ConversationEntity] {
        let all = try await suggestedEntities()
        guard !string.isEmpty else { return all }
        return all.filter { $0.title.localizedCaseInsensitiveContains(string) }
    }
}
