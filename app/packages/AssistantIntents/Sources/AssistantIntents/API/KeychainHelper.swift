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
///
/// Items are first looked up in the shared Keychain access group
/// (`com.cedricziel.assistant.shared`), falling back to the legacy
/// per-app scope for backwards compatibility during migration.
public struct KeychainHelper {
    /// The Keychain service name used by `flutter_secure_storage`.
    /// Defaults to the app's bundle identifier.
    private let service: String

    /// Shared Keychain access group used by both the main app and extensions.
    private let sharedAccessGroup: String?

    /// The Apple Team ID prefix (e.g. "ABCDE12345.") read from the app's
    /// entitlements at runtime. Returns an empty string if unavailable.
    public static var teamPrefix: String {
        guard let entitlements = Bundle.main.infoDictionary,
              let appId = entitlements["AppIdentifierPrefix"] as? String else {
            // Fallback: query Keychain for the team prefix.
            let query: [String: Any] = [
                kSecClass as String: kSecClassGenericPassword,
                kSecAttrAccount as String: "__team_prefix_probe__",
                kSecReturnAttributes as String: true,
            ]
            // Write a throwaway item to read back the access group.
            let addQuery: [String: Any] = [
                kSecClass as String: kSecClassGenericPassword,
                kSecAttrAccount as String: "__team_prefix_probe__",
                kSecValueData as String: Data("x".utf8),
            ]
            SecItemAdd(addQuery as CFDictionary, nil)
            var result: AnyObject?
            let status = SecItemCopyMatching(query as CFDictionary, &result)
            // Clean up probe.
            SecItemDelete(addQuery as CFDictionary)
            if status == errSecSuccess,
               let attrs = result as? [String: Any],
               let group = attrs[kSecAttrAccessGroup as String] as? String {
                // group is "TEAMID.bundleid" — extract "TEAMID."
                if let dotIndex = group.firstIndex(of: ".") {
                    return String(group[...dotIndex])
                }
            }
            return ""
        }
        return appId
    }

    public init(service: String? = nil, sharedAccessGroup: String? = nil) {
        self.service = service ?? Bundle.main.bundleIdentifier ?? "com.cedricziel.assistant"
        self.sharedAccessGroup = sharedAccessGroup
    }

    /// Reads a string value from the Keychain for the given key.
    ///
    /// Tries the shared access group first (if configured), then falls back
    /// to the default (per-app) scope.
    public func read(key: String) -> String? {
        // Try shared access group first.
        if let group = sharedAccessGroup {
            if let value = read(key: key, accessGroup: group) {
                return value
            }
        }
        // Fall back to default (no explicit access group — uses the app's own scope).
        return read(key: key, accessGroup: nil)
    }

    private func read(key: String, accessGroup: String?) -> String? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        if let accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        guard status == errSecSuccess, let data = result as? Data else {
            return nil
        }
        return String(data: data, encoding: .utf8)
    }

    /// Writes a string value to the Keychain under the shared access group.
    @discardableResult
    public func write(key: String, value: String) -> Bool {
        let data = Data(value.utf8)
        let accessGroup = sharedAccessGroup

        // Try to update an existing item first.
        var searchQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
        ]
        if let accessGroup {
            searchQuery[kSecAttrAccessGroup as String] = accessGroup
        }

        let updateAttrs: [String: Any] = [kSecValueData as String: data]
        let updateStatus = SecItemUpdate(searchQuery as CFDictionary, updateAttrs as CFDictionary)

        if updateStatus == errSecSuccess {
            return true
        }

        // Item doesn't exist — add it.
        var addQuery = searchQuery
        addQuery[kSecValueData as String] = data
        let addStatus = SecItemAdd(addQuery as CFDictionary, nil)
        return addStatus == errSecSuccess
    }

    /// Deletes a Keychain item.
    @discardableResult
    public func delete(key: String) -> Bool {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
        ]
        if let accessGroup = sharedAccessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        let status = SecItemDelete(query as CFDictionary)
        return status == errSecSuccess || status == errSecItemNotFound
    }

    /// Returns `true` if credentials are available.
    public var hasCredentials: Bool {
        serverURL != nil && authToken != nil
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
