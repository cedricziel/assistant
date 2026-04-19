import AppIntents

/// App Intent that returns the list of available personas.
public struct ListPersonasIntent: AppIntent {
    public static var title: LocalizedStringResource = "List Personas"
    public static var description = IntentDescription(
        "Get the list of available assistant personas."
    )

    public init() {}

    public func perform() async throws -> some IntentResult & ReturnsValue<[PersonaEntity]> & ProvidesDialog {
        let client = AssistantAPIClient.shared

        do {
            let personas = try await client.listPersonas()
            let entities = personas.map {
                PersonaEntity(id: $0.id, name: $0.name, summary: $0.description)
            }
            return .result(
                value: entities,
                dialog: "Found \(entities.count) persona\(entities.count == 1 ? "" : "s")."
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
