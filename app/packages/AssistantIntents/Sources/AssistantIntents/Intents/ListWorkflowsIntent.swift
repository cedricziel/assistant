import AppIntents

/// App Intent that returns the list of workflows.
public struct ListWorkflowsIntent: AppIntent {
    public static var title: LocalizedStringResource = "List Workflows"
    public static var description = IntentDescription(
        "Get the list of assistant workflows."
    )

    @Parameter(
        title: "Active Only",
        description: "Only return active workflows.",
        default: false
    )
    public var activeOnly: Bool

    public init() {}

    public func perform() async throws -> some IntentResult & ReturnsValue<[WorkflowEntity]> & ProvidesDialog {
        let client = AssistantAPIClient.shared

        do {
            var workflows = try await client.listWorkflows()
            if activeOnly {
                workflows = workflows.filter { $0.active }
            }
            let entities = workflows.map {
                WorkflowEntity(id: $0.id, name: $0.name, summary: $0.description, active: $0.active)
            }
            return .result(
                value: entities,
                dialog: "Found \(entities.count) workflow\(entities.count == 1 ? "" : "s")."
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
