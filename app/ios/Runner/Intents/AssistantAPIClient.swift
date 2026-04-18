import Foundation

/// Lightweight HTTP client for the assistant API, used by App Intents.
///
/// Reads credentials from the shared Keychain and calls the synchronous
/// `POST /api/quick-message` endpoint.
final class AssistantAPIClient {
    static let shared = AssistantAPIClient()

    private let keychain = KeychainHelper()
    private let timeoutSeconds: TimeInterval = 25

    struct QuickMessageResponse: Decodable {
        let conversation_id: String
        let message_id: String?
        let answer: String
    }

    enum APIError: Error, LocalizedError {
        case noCredentials
        case invalidURL
        case timeout
        case serverError(Int, String)
        case networkError(Error)

        var errorDescription: String? {
            switch self {
            case .noCredentials:
                return "No server credentials found. Open the app and connect to your assistant server first."
            case .invalidURL:
                return "Invalid server URL configured."
            case .timeout:
                return "The request timed out. The assistant may still be working."
            case .serverError(let code, let message):
                return "Server error (\(code)): \(message)"
            case .networkError(let error):
                return "Network error: \(error.localizedDescription)"
            }
        }
    }

    /// Sends a message to the assistant and returns the complete response.
    func quickMessage(_ message: String) async throws -> QuickMessageResponse {
        guard let serverURL = keychain.serverURL else {
            throw APIError.noCredentials
        }

        guard let url = URL(string: "\(serverURL)/api/quick-message") else {
            throw APIError.invalidURL
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.timeoutInterval = timeoutSeconds

        if let token = keychain.authToken {
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }

        let body = ["message": message]
        request.httpBody = try JSONSerialization.data(withJSONObject: body)

        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await URLSession.shared.data(for: request)
        } catch let error as URLError where error.code == .timedOut {
            throw APIError.timeout
        } catch {
            throw APIError.networkError(error)
        }

        guard let httpResponse = response as? HTTPURLResponse else {
            throw APIError.networkError(URLError(.badServerResponse))
        }

        guard (200...299).contains(httpResponse.statusCode) else {
            let errorBody = String(data: data, encoding: .utf8) ?? "Unknown error"
            throw APIError.serverError(httpResponse.statusCode, errorBody)
        }

        return try JSONDecoder().decode(QuickMessageResponse.self, from: data)
    }
}
