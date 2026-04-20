import Foundation

/// Lightweight HTTP client for the assistant API, used by App Intents.
///
/// Reads credentials from the shared Keychain and calls the assistant
/// server's REST API endpoints.
public final class AssistantAPIClient {
    public static let shared = AssistantAPIClient()

    private let keychain: KeychainHelper
    private let timeoutSeconds: TimeInterval = 25

    public init() {
        // Use shared Keychain access group so extensions can read credentials
        // written by the main app.
        self.keychain = KeychainHelper(
            service: "com.cedricziel.assistant",
            sharedAccessGroup: "\(KeychainHelper.teamPrefix)com.cedricziel.assistant.shared"
        )
    }

    // MARK: - Response types

    public struct QuickMessageResponse: Decodable {
        public let conversationId: String
        public let messageId: String?
        public let answer: String

        private enum CodingKeys: String, CodingKey {
            case conversationId = "conversation_id"
            case messageId = "message_id"
            case answer
        }
    }

    public struct PersonaSummary: Decodable, Identifiable {
        public let id: String
        public let name: String
        public let description: String
    }

    public struct WorkflowSummary: Decodable, Identifiable {
        public let id: String
        public let name: String
        public let description: String
        public let active: Bool

        private enum CodingKeys: String, CodingKey {
            case id, name, description, active
        }
    }

    public struct ConversationSummary: Decodable, Identifiable {
        public let id: String
        public let title: String?
        public let updatedAt: String?

        private enum CodingKeys: String, CodingKey {
            case id
            case title
            case updatedAt = "updated_at"
        }
    }

    public struct WorkflowRunPreview: Decodable {
        public let workflowId: String
        public let runId: String
        public let status: String

        private enum CodingKeys: String, CodingKey {
            case workflowId = "workflow_id"
            case runId = "run_id"
            case status
        }
    }

    public struct AttachmentResponse: Decodable {
        public let id: String
        public let filename: String
        public let mimeType: String
        public let sizeBytes: Int64
        public let url: String

        private enum CodingKeys: String, CodingKey {
            case id
            case filename
            case mimeType = "mime_type"
            case sizeBytes = "size_bytes"
            case url
        }
    }

    // MARK: - Errors

    public enum APIError: Error, LocalizedError {
        case noCredentials
        case invalidURL
        case timeout
        case serverError(Int)
        case networkError(Error)
        case notFound

        public var errorDescription: String? {
            switch self {
            case .noCredentials:
                return "No server credentials found. Open the app and connect to your assistant server first."
            case .invalidURL:
                return "Invalid server URL configured."
            case .timeout:
                return "The request timed out. The assistant may still be working."
            case .serverError(let code):
                return "Server returned status \(code)."
            case .networkError:
                return "A network error occurred."
            case .notFound:
                return "The requested resource was not found."
            }
        }
    }

    // MARK: - Quick message

    /// Sends a message to the assistant and returns the complete response.
    public func quickMessage(
        _ message: String,
        personaId: String? = nil,
        context: String? = nil
    ) async throws -> QuickMessageResponse {
        var body: [String: Any] = ["message": message]
        if let personaId { body["persona_id"] = personaId }
        if let context { body["context"] = context }
        return try await post(path: "/api/quick-message", body: body)
    }

    // MARK: - Personas

    /// Lists all available personas.
    public func listPersonas() async throws -> [PersonaSummary] {
        try await get(path: "/api/personas")
    }

    // MARK: - Workflows

    /// Lists all workflows.
    public func listWorkflows() async throws -> [WorkflowSummary] {
        try await get(path: "/api/workflows")
    }

    /// Triggers a manual test run of a workflow.
    public func triggerWorkflow(id: String) async throws -> WorkflowRunPreview {
        try await post(path: "/api/workflows/\(id)/test-run", body: [:])
    }

    // MARK: - Conversations

    /// Lists recent conversations.
    public func listConversations() async throws -> [ConversationSummary] {
        try await get(path: "/api/conversations")
    }

    /// Creates a new conversation and returns its summary.
    public func createConversation(title: String? = nil) async throws -> ConversationSummary {
        var body: [String: Any] = [:]
        if let title { body["title"] = title }
        return try await post(path: "/api/conversations", body: body)
    }

    /// Sends a message to a conversation. The server responds with SSE but
    /// this method validates the initial HTTP status and returns without
    /// waiting for the stream to complete.
    public func sendMessage(
        conversationId: String,
        message: String,
        attachmentIds: [String] = []
    ) async throws {
        var body: [String: Any] = ["message": message]
        if !attachmentIds.isEmpty {
            body["attachment_ids"] = attachmentIds
        }

        let base = try baseURL()
        let path = "/api/conversations/\(conversationId)/messages"
        guard let url = URL(string: "\(base)\(path)") else {
            throw APIError.invalidURL
        }

        var request = makeRequest(url: url, method: "POST")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: body)

        // Use bytes to get the response headers without consuming the full SSE stream.
        let (bytes, response) = try await URLSession.shared.bytes(for: request)
        try validateResponse(response)
        // Cancel the stream — we only needed the status code.
        bytes.task.cancel()
    }

    // MARK: - Attachments

    /// Uploads a file as a multipart attachment to a conversation.
    public func uploadAttachment(
        conversationId: String,
        fileData: Data,
        filename: String,
        mimeType: String
    ) async throws -> AttachmentResponse {
        let base = try baseURL()
        let path = "/api/conversations/\(conversationId)/attachments"
        guard let url = URL(string: "\(base)\(path)") else {
            throw APIError.invalidURL
        }

        let boundary = UUID().uuidString
        var request = makeRequest(url: url, method: "POST")
        request.setValue(
            "multipart/form-data; boundary=\(boundary)",
            forHTTPHeaderField: "Content-Type"
        )

        let safeFilename = filename
            .replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"")
            .replacingOccurrences(of: "\r", with: "")
            .replacingOccurrences(of: "\n", with: "")
        var body = Data()
        body.append("--\(boundary)\r\n")
        body.append("Content-Disposition: form-data; name=\"file\"; filename=\"\(safeFilename)\"\r\n")
        body.append("Content-Type: \(mimeType)\r\n\r\n")
        body.append(fileData)
        body.append("\r\n--\(boundary)--\r\n")
        request.httpBody = body

        let (data, response) = try await execute(request)
        try validateResponse(response)
        return try JSONDecoder().decode(AttachmentResponse.self, from: data)
    }

    /// Convenience overload that reads the file at `fileURL` into memory
    /// and delegates to the `Data`-based upload method.
    public func uploadAttachment(
        conversationId: String,
        fileURL: URL,
        mimeType: String
    ) async throws -> AttachmentResponse {
        let fileData = try Data(contentsOf: fileURL)
        let filename = fileURL.lastPathComponent
        return try await uploadAttachment(
            conversationId: conversationId,
            fileData: fileData,
            filename: filename,
            mimeType: mimeType
        )
    }

    // MARK: - Personas (active)

    /// Switches the server's active persona.
    public func switchPersona(id: String) async throws {
        try await postIgnoringBody(
            path: "/api/personas/active",
            body: ["id": id]
        )
    }

    // MARK: - Internal HTTP helpers

    private func baseURL() throws -> String {
        guard let url = keychain.serverURL else {
            throw APIError.noCredentials
        }
        return url
    }

    private func makeRequest(url: URL, method: String) -> URLRequest {
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.timeoutInterval = timeoutSeconds

        if let token = keychain.authToken {
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        return request
    }

    private func get<T: Decodable>(path: String) async throws -> T {
        let base = try baseURL()
        guard let url = URL(string: "\(base)\(path)") else {
            throw APIError.invalidURL
        }

        let request = makeRequest(url: url, method: "GET")
        let (data, response) = try await execute(request)
        try validateResponse(response)
        return try JSONDecoder().decode(T.self, from: data)
    }

    private func post<T: Decodable>(path: String, body: [String: Any]) async throws -> T {
        let base = try baseURL()
        guard let url = URL(string: "\(base)\(path)") else {
            throw APIError.invalidURL
        }

        var request = makeRequest(url: url, method: "POST")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: body)

        let (data, response) = try await execute(request)
        try validateResponse(response)
        return try JSONDecoder().decode(T.self, from: data)
    }

    private func postIgnoringBody(path: String, body: [String: Any]) async throws {
        let base = try baseURL()
        guard let url = URL(string: "\(base)\(path)") else {
            throw APIError.invalidURL
        }

        var request = makeRequest(url: url, method: "POST")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: body)

        let (_, response) = try await execute(request)
        try validateResponse(response)
    }

    private func execute(_ request: URLRequest) async throws -> (Data, URLResponse) {
        do {
            return try await URLSession.shared.data(for: request)
        } catch let error as URLError where error.code == .timedOut {
            throw APIError.timeout
        } catch {
            throw APIError.networkError(error)
        }
    }

    private func validateResponse(_ response: URLResponse) throws {
        guard let httpResponse = response as? HTTPURLResponse else {
            throw APIError.networkError(URLError(.badServerResponse))
        }
        if httpResponse.statusCode == 404 {
            throw APIError.notFound
        }
        guard (200...299).contains(httpResponse.statusCode) else {
            throw APIError.serverError(httpResponse.statusCode)
        }
    }
}

// MARK: - Data helpers

private extension Data {
    mutating func append(_ string: String) {
        if let data = string.data(using: .utf8) {
            append(data)
        }
    }
}
