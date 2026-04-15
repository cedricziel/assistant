import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for ServerCapabilities
void main() {
  final instance = ServerCapabilitiesBuilder();
  // TODO add properties to the builder and call build()

  group(ServerCapabilities, () {
    // Whether the server can serve TTS audio for messages (TTS configured).
    // bool voiceReceive
    test('to test the property `voiceReceive`', () async {
      // TODO
    });

    // Whether the server can accept voice messages (STT configured).
    // bool voiceSend
    test('to test the property `voiceSend`', () async {
      // TODO
    });

  });
}
