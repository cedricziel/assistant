import AppIntents

/// An App Entity representing an assistant persona.
public struct PersonaEntity: AppEntity {
    public static var typeDisplayRepresentation = TypeDisplayRepresentation(name: "Persona")
    public static var defaultQuery = PersonaEntityQuery()

    public var id: String
    public var name: String
    public var summary: String

    public var displayRepresentation: DisplayRepresentation {
        DisplayRepresentation(title: "\(name)", subtitle: "\(summary)")
    }

    public init(id: String, name: String, summary: String) {
        self.id = id
        self.name = name
        self.summary = summary
    }
}

/// Query for persona entities backed by the assistant REST API.
public struct PersonaEntityQuery: EntityQuery {
    public init() {}

    public func entities(for identifiers: [String]) async throws -> [PersonaEntity] {
        let all = try await suggestedEntities()
        return all.filter { identifiers.contains($0.id) }
    }

    public func suggestedEntities() async throws -> [PersonaEntity] {
        do {
            let personas = try await AssistantAPIClient.shared.listPersonas()
            return personas.map {
                PersonaEntity(id: $0.id, name: $0.name, summary: $0.description)
            }
        } catch {
            return []
        }
    }
}

extension PersonaEntityQuery: EntityStringQuery {
    public func entities(matching string: String) async throws -> [PersonaEntity] {
        let all = try await suggestedEntities()
        guard !string.isEmpty else { return all }
        return all.filter { $0.name.localizedCaseInsensitiveContains(string) }
    }
}
