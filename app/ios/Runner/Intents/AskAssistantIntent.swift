import AppIntents

/// App Intent that accepts a spoken or typed question, sends it to the
/// assistant server via `POST /api/quick-message`, and returns the answer
/// as a Siri dialog.
///
/// Discoverable via Shortcuts app and configurable as an Action Button target.
@available(iOS 16.0, *)
struct AskAssistantIntent: AppIntent {
    static var title: LocalizedStringResource = "Ask Assistant"
    static var description = IntentDescription(
        "Send a question to your assistant and hear the answer."
    )

    @Parameter(
        title: "Question",
        requestValueDialog: "What would you like to ask?"
    )
    var question: String

    func perform() async throws -> some IntentResult & ProvidesDialog {
        let client = AssistantAPIClient.shared

        do {
            let response = try await client.quickMessage(question)
            return .result(dialog: "\(response.answer)")
        } catch AssistantAPIClient.APIError.noCredentials {
            return .result(
                dialog: "Please open the app and connect to your assistant server first."
            )
        } catch AssistantAPIClient.APIError.timeout {
            return .result(
                dialog: "I'm still working on that. Check the app for the full answer."
            )
        } catch AssistantAPIClient.APIError.networkError {
            return .result(
                dialog: "I couldn't reach your assistant server. Please check your connection."
            )
        } catch {
            return .result(
                dialog: "Something went wrong: \(error.localizedDescription)"
            )
        }
    }
}
