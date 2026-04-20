import FlutterMacOS
import AssistantIntents

/// Method channel that writes credentials to the shared Keychain access group
/// so that app extensions (share extension) can read them.
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

                if let url = serverUrl {
                    keychain.write(key: "assistant_siri_server_url", value: url)
                } else {
                    keychain.delete(key: "assistant_siri_server_url")
                }

                if let token = authToken {
                    keychain.write(key: "assistant_siri_auth_token", value: token)
                } else {
                    keychain.delete(key: "assistant_siri_auth_token")
                }

                result(true)

            default:
                result(FlutterMethodNotImplemented)
            }
        }
    }
}
