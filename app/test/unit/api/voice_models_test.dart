import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/api/models/stream_event.dart';

void main() {
  group('ServerCapabilities', () {
    test('fromJson parses both flags as true', () {
      final caps = ServerCapabilities.fromJson({
        'voice_send': true,
        'voice_receive': true,
      });
      expect(caps.voiceSend, isTrue);
      expect(caps.voiceReceive, isTrue);
    });

    test('fromJson parses both flags as false', () {
      final caps = ServerCapabilities.fromJson({
        'voice_send': false,
        'voice_receive': false,
      });
      expect(caps.voiceSend, isFalse);
      expect(caps.voiceReceive, isFalse);
    });

    test('fromJson defaults missing flags to false', () {
      final caps = ServerCapabilities.fromJson({});
      expect(caps.voiceSend, isFalse);
      expect(caps.voiceReceive, isFalse);
    });

    test('fromJson handles partial flags', () {
      final caps = ServerCapabilities.fromJson({'voice_send': true});
      expect(caps.voiceSend, isTrue);
      expect(caps.voiceReceive, isFalse);
    });

    test('disabled constant has both flags false', () {
      expect(ServerCapabilities.disabled.voiceSend, isFalse);
      expect(ServerCapabilities.disabled.voiceReceive, isFalse);
    });
  });

  group('AudioReadyEvent', () {
    test('stores audioId', () {
      const event = AudioReadyEvent('abc-123');
      expect(event.audioId, equals('abc-123'));
    });

    test('fromJson parses audio_id', () {
      final event = AudioReadyEvent.fromJson({'audio_id': 'my-uuid'});
      expect(event.audioId, equals('my-uuid'));
    });

    test('fromJson defaults missing audio_id to empty string', () {
      final event = AudioReadyEvent.fromJson({});
      expect(event.audioId, equals(''));
    });

    test('is a StreamEvent subtype', () {
      const StreamEvent event = AudioReadyEvent('id');
      expect(event, isA<AudioReadyEvent>());
    });

    test('pattern match works in switch', () {
      const StreamEvent event = AudioReadyEvent('play-me');
      String? gotId;
      switch (event) {
        case AudioReadyEvent(:final audioId):
          gotId = audioId;
        default:
          break;
      }
      expect(gotId, equals('play-me'));
    });
  });
}
