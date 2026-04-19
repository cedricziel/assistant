import AppIntents

/// App Intent that accepts a spoken or typed question, sends it to the
/// assistant server via `POST /api/quick-message`, and returns the answer
/// as a Siri dialog.
///
/// Optionally accepts a persona to route the question and context text
/// to include with the message.
public struct AskAssistantIntent: AppIntent {
    public static var title: LocalizedStringResource = "Ask Assistant"
    public static var description = IntentDescription(
        "Send a question to your assistant and hear the answer."
    )

    @Parameter(
        title: "Question",
        requestValueDialog: "What would you like to ask?"
    )
    public var question: String

    @Parameter(
        title: "Persona",
        description: "Which persona to ask. Uses the default if not specified."
    )
    public var persona: PersonaEntity?

    @Parameter(
        title: "Context",
        description: "Additional context to include (e.g. clipboard text, file contents)."
    )
    public var context: String?

    public init() {}

    public func perform() async throws -> some IntentResult & ProvidesDialog {
        let client = AssistantAPIClient.shared

        do {
            let response = try await client.quickMessage(
                question,
                personaId: persona?.id,
                context: context
            )
            return .result(dialog: "\(response.answer)")
        } catch AssistantAPIClient.APIError.noCredentials {
            return .result(
                dialog: "Please open the app and connect to your assistant server first."
            )
        } catch AssistantAPIClient.APIError.timeout {
            return .result(
                dialog: "I'm still working on that. Check the app for the full answer."
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
