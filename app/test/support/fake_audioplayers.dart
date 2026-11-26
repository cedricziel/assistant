import 'dart:async';
import 'dart:typed_data';

import 'package:audioplayers_platform_interface/audioplayers_platform_interface.dart';

/// Installs platform fakes so `AudioPlayer()` can be constructed in widget
/// tests without native channels. Call from `setUp`.
void installFakeAudioplayers() {
  AudioplayersPlatformInterface.instance = _FakeAudioplayersPlatform();
  GlobalAudioplayersPlatformInterface.instance =
      _FakeGlobalAudioplayersPlatform();
}

class _FakeGlobalAudioplayersPlatform
    extends GlobalAudioplayersPlatformInterface {
  @override
  Future<void> init() async {}
  @override
  Future<void> setGlobalAudioContext(AudioContext ctx) async {}
  @override
  Future<void> emitGlobalLog(String message) async {}
  @override
  Future<void> emitGlobalError(String code, String message) async {}
  @override
  Stream<GlobalAudioEvent> getGlobalEventStream() => const Stream.empty();
}

class _FakeAudioplayersPlatform extends AudioplayersPlatformInterface {
  final _controllers = <String, StreamController<AudioEvent>>{};

  StreamController<AudioEvent> _ctrl(String id) =>
      _controllers.putIfAbsent(id, () => StreamController.broadcast());

  @override
  Future<void> create(String playerId) async {
    _ctrl(playerId);
  }

  @override
  Stream<AudioEvent> getEventStream(String playerId) => _ctrl(playerId).stream;

  @override
  Future<void> dispose(String playerId) async {
    await _controllers.remove(playerId)?.close();
  }

  @override
  Future<void> setSourceUrl(
    String playerId,
    String url, {
    bool? isLocal,
    String? mimeType,
  }) async {}
  @override
  Future<void> setSourceBytes(
    String playerId,
    Uint8List bytes, {
    String? mimeType,
  }) async {}
  @override
  Future<void> resume(String playerId) async {}
  @override
  Future<void> pause(String playerId) async {}
  @override
  Future<void> stop(String playerId) async {}
  @override
  Future<void> release(String playerId) async {}
  @override
  Future<void> seek(String playerId, Duration position) async {}
  @override
  Future<void> setVolume(String playerId, double volume) async {}
  @override
  Future<void> setBalance(String playerId, double balance) async {}
  @override
  Future<void> setReleaseMode(String playerId, ReleaseMode releaseMode) async {}
  @override
  Future<void> setPlaybackRate(String playerId, double playbackRate) async {}
  @override
  Future<void> setAudioContext(
    String playerId,
    AudioContext audioContext,
  ) async {}
  @override
  Future<void> setPlayerMode(String playerId, PlayerMode playerMode) async {}
  @override
  Future<int?> getDuration(String playerId) async => null;
  @override
  Future<int?> getCurrentPosition(String playerId) async => null;
  @override
  Future<void> emitLog(String playerId, String message) async {}
  @override
  Future<void> emitError(String playerId, String code, String message) async {}
}
