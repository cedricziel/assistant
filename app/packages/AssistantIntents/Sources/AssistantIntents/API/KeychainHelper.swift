import Foundation
import Security

/// Reads credentials written by `flutter_secure_storage` from the Keychain.
///
/// `flutter_secure_storage` stores items with:
/// - `kSecAttrService` = bundle identifier (default)
/// - `kSecAttrAccount` = the key name passed to `write(key:value:)`
///
/// The well-known keys are:
/// - `assistant_siri_server_url` — base URL of the assistant server
/// - `assistant_siri_auth_token` — bearer token for API auth
public struct KeychainHelper {
    /// The Keychain service name used by `flutter_secure_storage`.
    /// Defaults to the app's bundle identifier.
    private let service: String

    public init(service: String? = nil) {
        self.service = service ?? Bundle.main.bundleIdentifier ?? "com.cedricziel.assistant"
    }

    /// Reads a string value from the Keychain for the given key.
    public func read(key: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        guard status == errSecSuccess, let data = result as? Data else {
            return nil
        }
        return String(data: data, encoding: .utf8)
    }

    /// Returns the server URL stored by the Flutter app, or `nil` if not set.
    public var serverURL: String? {
        read(key: "assistant_siri_server_url")
    }

    /// Returns the auth token stored by the Flutter app, or `nil` if not set.
    public var authToken: String? {
        read(key: "assistant_siri_auth_token")
    }
}
