import 'dart:async';

import 'package:audioplayers_platform_interface/audioplayers_platform_interface.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/audio_player_widget.dart';
import 'package:assistant_app/features/chat/voice_recorder_button.dart';

// ---------------------------------------------------------------------------
// Fake AudioplayersPlatformInterface — avoids all native channel calls and
// auto-emits "prepared" so that _completePrepared() resolves immediately.
// ---------------------------------------------------------------------------

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
    _ctrl(playerId); // ensure controller exists
  }

  @override
  Stream<AudioEvent> getEventStream(String playerId) => _ctrl(playerId).stream;

  @override
  Future<void> dispose(String playerId) async {
    await _controllers.remove(playerId)?.close();
  }

  // Emit "prepared" so _completePrepared() resolves immediately.
  void _emitPrepared(String playerId) {
    _ctrl(playerId).add(
      const AudioEvent(eventType: AudioEventType.prepared, isPrepared: true),
    );
  }

  @override
  Future<void> setSourceUrl(
    String playerId,
    String url, {
    bool? isLocal,
    String? mimeType,
  }) async {
    _emitPrepared(playerId);
  }

  @override
  Future<void> setSourceBytes(
    String playerId,
    Uint8List bytes, {
    String? mimeType,
  }) async {
    _emitPrepared(playerId);
  }

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

// ---------------------------------------------------------------------------
// Mock handler for the record package channel so AudioRecorder() construction
// succeeds and hasPermission() returns a controllable value.
// ---------------------------------------------------------------------------

void _setRecordChannelMock({required bool permissionGranted}) {
  TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
      .setMockMethodCallHandler(
        const MethodChannel('com.llfbandit.record/messages'),
        (call) async {
          if (call.method == 'hasPermission') return permissionGranted;
          return null;
        },
      );
}

void _clearRecordChannelMock() {
  TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
      .setMockMethodCallHandler(
        const MethodChannel('com.llfbandit.record/messages'),
        null,
      );
}

// ---------------------------------------------------------------------------
// AudioPlayerWidget tests
// ---------------------------------------------------------------------------

void main() {
  late _FakeAudioplayersPlatform fakePlayer;

  setUp(() {
    fakePlayer = _FakeAudioplayersPlatform();
    AudioplayersPlatformInterface.instance = fakePlayer;
    GlobalAudioplayersPlatformInterface.instance =
        _FakeGlobalAudioplayersPlatform();
    _setRecordChannelMock(permissionGranted: false);
  });

  tearDown(_clearRecordChannelMock);

  group('AudioPlayerWidget', () {
    testWidgets('initially shows a play icon', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AudioPlayerWidget(
              fetchAudio: () async => Uint8List.fromList([1, 2, 3]),
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.play_circle_outlined), findsOneWidget);
      expect(find.byIcon(Icons.stop_circle_outlined), findsNothing);
      expect(find.byType(CircularProgressIndicator), findsNothing);
    });

    testWidgets('shows spinner while audio is being fetched', (tester) async {
      // Use a Completer that never completes so the fetch stays in-flight.
      final completer = Completer<Uint8List?>();

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AudioPlayerWidget(fetchAudio: () => completer.future),
          ),
        ),
      );

      // Tap play — triggers the fetch.
      await tester.tap(find.byType(IconButton));
      await tester.pump(); // setState(_isLoading = true) takes effect

      expect(
        find.byType(CircularProgressIndicator),
        findsOneWidget,
        reason: 'spinner must appear while fetch is in-flight',
      );
      expect(find.byIcon(Icons.play_circle_outlined), findsNothing);

      // Complete to clean up the pending future before the test ends.
      completer.complete(null);
      await tester.pump();
    });

    testWidgets('shows play icon again when fetchAudio returns null', (
      tester,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: AudioPlayerWidget(fetchAudio: () async => null)),
        ),
      );

      await tester.tap(find.byType(IconButton));
      // runAsync lets real async futures resolve; pump applies setState changes.
      await tester.runAsync(() => Future<void>.delayed(Duration.zero));
      await tester.pump();

      expect(
        find.byIcon(Icons.play_circle_outlined),
        findsOneWidget,
        reason: 'null audio — widget returns to idle play state',
      );
      expect(find.byType(CircularProgressIndicator), findsNothing);
    });

    testWidgets('caches bytes — fetchAudio called only once across two taps', (
      tester,
    ) async {
      int callCount = 0;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AudioPlayerWidget(
              fetchAudio: () async {
                callCount++;
                return Uint8List.fromList([1, 2, 3]);
              },
            ),
          ),
        ),
      );

      // Allow AudioPlayer._create() to complete (fake platform → no real channels).
      await tester.pump();

      // First tap: fetches audio and plays. The fake platform emits "prepared"
      // immediately, so _completePrepared resolves without a timeout.
      await tester.tap(find.byType(IconButton));
      await tester.runAsync(() async {
        // Allow the full async chain (fetchAudio → setSource → prepared event →
        // resume → setState) to complete.
        await Future<void>.delayed(const Duration(milliseconds: 50));
      });
      await tester
          .pump(); // rebuild after setState(_isPlaying=true, _isLoading=false)

      // Second tap: _cachedBytes is set — fetchAudio must NOT be called again.
      // Widget shows a stop or play IconButton; either way tap() succeeds.
      await tester.tap(find.byType(IconButton));
      await tester.runAsync(() => Future<void>.delayed(Duration.zero));
      await tester.pump();

      expect(
        callCount,
        equals(1),
        reason: 'fetchAudio must not be called again after first fetch',
      );
    });
  });

  // ---------------------------------------------------------------------------
  // VoiceRecorderButton tests
  // ---------------------------------------------------------------------------

  group('VoiceRecorderButton', () {
    testWidgets('initially shows the mic icon and no stop icon', (
      tester,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: VoiceRecorderButton(
              onRecordingComplete: (bytes, mime) {},
              onError: (_) {},
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.mic_none_outlined), findsOneWidget);
      expect(find.byIcon(Icons.stop_circle_outlined), findsNothing);
    });

    testWidgets('calls onError when microphone permission is denied', (
      tester,
    ) async {
      // Channel mock returns false for hasPermission → maps to the
      // permission-denied code path in _start().
      String? capturedError;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: VoiceRecorderButton(
              onRecordingComplete: (bytes, mime) {},
              onError: (e) => capturedError = e,
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.mic_none_outlined));
      // runAsync lets real platform-channel futures resolve before we check.
      await tester.runAsync(() => Future<void>.delayed(Duration.zero));
      await tester.pump();

      // Permission denied → onError must have been called.
      expect(
        capturedError,
        isNotNull,
        reason: 'onError must be called when recording cannot start',
      );

      // After an error the mic icon is still shown (not the stop button).
      expect(find.byIcon(Icons.mic_none_outlined), findsOneWidget);
      expect(find.byIcon(Icons.stop_circle_outlined), findsNothing);
    });
  });
}
