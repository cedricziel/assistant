import AppIntents

/// App Intent that triggers an assistant workflow by name.
public struct RunWorkflowIntent: AppIntent {
    public static var title: LocalizedStringResource = "Run Workflow"
    public static var description = IntentDescription(
        "Trigger an assistant workflow."
    )

    @Parameter(
        title: "Workflow",
        description: "The workflow to run."
    )
    public var workflow: WorkflowEntity

    public init() {}

    public func perform() async throws -> some IntentResult & ProvidesDialog {
        let client = AssistantAPIClient.shared

        do {
            let run = try await client.triggerWorkflow(id: workflow.id)
            return .result(
                dialog: "Workflow '\(workflow.name)' started. Run ID: \(run.runId)"
            )
        } catch AssistantAPIClient.APIError.noCredentials {
            return .result(
                dialog: "Please open the app and connect to your assistant server first."
            )
        } catch AssistantAPIClient.APIError.notFound {
            return .result(
                dialog: "Workflow not found. It may have been deleted."
            )
        } catch is AssistantAPIClient.APIError {
            return .result(
                dialog: "I couldn't reach your assistant server. Please check your connection."
            )
        } catch {
            return .result(
                dialog: "Something went wrong. Please try again later."
            )
        }
    }
}
