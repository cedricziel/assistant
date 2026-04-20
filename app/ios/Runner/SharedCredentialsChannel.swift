import Flutter
import AssistantIntents

/// Method channel that writes credentials to the shared Keychain access group
/// so that app extensions (share extension, Siri) can read them.
final class SharedCredentialsChannel {
    static let channelName = "com.cedricziel.assistant/shared_credentials"

    static func register(with messenger: FlutterBinaryMessenger) {
        let channel = FlutterMethodChannel(name: channelName, binaryMessenger: messenger)
        channel.setMethodCallHandler { call, result in
            switch call.method {
            case "syncCredentials":
                guard let args = call.arguments as? [String: Any] else {
                    result(FlutterError(code: "INVALID_ARGS", message: "Expected a map", details: nil))
                    return
                }
                let serverUrl = args["serverUrl"] as? String
                let authToken = args["authToken"] as? String

                let keychain = KeychainHelper(
                    service: "com.cedricziel.assistant",
                    sharedAccessGroup: "\(KeychainHelper.teamPrefix)com.cedricziel.assistant.shared"
                )

                let serverOK: Bool
                if let url = serverUrl, !url.isEmpty {
                    serverOK = keychain.write(key: "assistant_siri_server_url", value: url)
                } else {
                    serverOK = keychain.delete(key: "assistant_siri_server_url")
                }

                let tokenOK: Bool
                if let token = authToken, !token.isEmpty {
                    tokenOK = keychain.write(key: "assistant_siri_auth_token", value: token)
                } else {
                    tokenOK = keychain.delete(key: "assistant_siri_auth_token")
                }

                guard serverOK && tokenOK else {
                    result(FlutterError(
                        code: "KEYCHAIN_SYNC_FAILED",
                        message: "Failed to sync credentials to the shared Keychain access group",
                        details: nil
                    ))
                    return
                }
                result(true)

            default:
                result(FlutterMethodNotImplemented)
            }
        }
    }
}
