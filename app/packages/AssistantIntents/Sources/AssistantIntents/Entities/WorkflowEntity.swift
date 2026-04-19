import AppIntents

/// An App Entity representing an assistant workflow.
public struct WorkflowEntity: AppEntity {
    public static var typeDisplayRepresentation = TypeDisplayRepresentation(name: "Workflow")
    public static var defaultQuery = WorkflowEntityQuery()

    public var id: String
    public var name: String
    public var summary: String
    public var active: Bool

    public var displayRepresentation: DisplayRepresentation {
        let status = active ? "Active" : "Inactive"
        return DisplayRepresentation(title: "\(name)", subtitle: "\(status)")
    }

    public init(id: String, name: String, summary: String, active: Bool) {
        self.id = id
        self.name = name
        self.summary = summary
        self.active = active
    }
}

/// Query for workflow entities backed by the assistant REST API.
public struct WorkflowEntityQuery: EntityQuery {
    public init() {}

    public func entities(for identifiers: [String]) async throws -> [WorkflowEntity] {
        let all = try await suggestedEntities()
        return all.filter { identifiers.contains($0.id) }
    }

    public func suggestedEntities() async throws -> [WorkflowEntity] {
        do {
            let workflows = try await AssistantAPIClient.shared.listWorkflows()
            return workflows.map {
                WorkflowEntity(id: $0.id, name: $0.name, summary: $0.description, active: $0.active)
            }
        } catch {
            return []
        }
    }
}

extension WorkflowEntityQuery: EntityStringQuery {
    public func entities(matching string: String) async throws -> [WorkflowEntity] {
        let all = try await suggestedEntities()
        guard !string.isEmpty else { return all }
        return all.filter { $0.name.localizedCaseInsensitiveContains(string) }
    }
}
