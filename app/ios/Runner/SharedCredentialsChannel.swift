import Flutter
import AssistantIntents

/// Method channel that exposes the Apple Team ID prefix to Dart so that
/// `flutter_secure_storage` can write to the shared Keychain access group
/// using `IOSOptions(groupId:)`.
final class SharedCredentialsChannel {
    static let channelName = "com.cedricziel.assistant/shared_credentials"

    static func register(with messenger: FlutterBinaryMessenger) {
        let channel = FlutterMethodChannel(name: channelName, binaryMessenger: messenger)
        channel.setMethodCallHandler { call, result in
            switch call.method {
            case "getTeamPrefix":
                result(KeychainHelper.teamPrefix)

            default:
                result(FlutterMethodNotImplemented)
            }
        }
    }
}
