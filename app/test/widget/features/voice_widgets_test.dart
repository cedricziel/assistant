import 'dart:async';
import 'dart:io';

import 'package:audioplayers_platform_interface/audioplayers_platform_interface.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:record/record.dart';

import 'package:assistant_app/api/connectivity_provider.dart';
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

// Mock path_provider so getTemporaryDirectory() resolves in test environment.
// The newer macOS/iOS plugin uses the _foundation channel name.
void _setPathProviderMock() {
  for (final ch in const [
    'plugins.flutter.io/path_provider_foundation',
    'plugins.flutter.io/path_provider',
  ]) {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(MethodChannel(ch), (call) async {
          if (call.method == 'getTemporaryDirectory') {
            return Directory.systemTemp.path;
          }
          return null;
        });
  }
}

void _clearPathProviderMock() {
  for (final ch in const [
    'plugins.flutter.io/path_provider_foundation',
    'plugins.flutter.io/path_provider',
  ]) {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(MethodChannel(ch), null);
  }
}

void _clearRecordChannelMock() {
  TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
      .setMockMethodCallHandler(
        const MethodChannel('com.llfbandit.record/messages'),
        null,
      );
}

/// Fake [AudioRecorder] driving a full recording cycle without native calls.
///
/// Permission is granted, all encoders report as supported, [start] succeeds,
/// and [stop] returns [stopPath] (the file the caller pre-populated with test
/// bytes).
///
/// This injects through [VoiceRecorderButton.audioRecorder] rather than mocking
/// `com.llfbandit.record/messages`. As of record 7.x, `start()` also subscribes
/// to a per-instance `EventChannel` (`com.llfbandit.record/events/<uuid>`) whose
/// name is not known ahead of time, so it cannot be mocked by channel name — a
/// method-channel mock alone leaves that stream unhandled and start() throws
/// MissingPluginException.
class _FakeAudioRecorder implements AudioRecorder {
  _FakeAudioRecorder(this.stopPath);

  final String stopPath;

  @override
  Future<bool> hasPermission({bool request = true}) async => true;

  @override
  Future<bool> isEncoderSupported(AudioEncoder encoder) async => true;

  @override
  Future<void> start(RecordConfig config, {required String path}) async {}

  @override
  Future<String?> stop() async => stopPath;

  @override
  Future<void> dispose() async {}

  // Any other member of AudioRecorder is unused by VoiceRecorderButton; reaching
  // one is a bug in the test rather than something to silently return null for.
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
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

  tearDown(() {
    _clearRecordChannelMock();
    _clearPathProviderMock();
  });

  group('AudioPlayerWidget', () {
    testWidgets('initially shows a play icon', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AudioPlayerWidget(
              fetchAudio: () async => (
                bytes: Uint8List.fromList([1, 2, 3]),
                mimeType: 'audio/mpeg',
              ),
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
      final completer = Completer<({Uint8List bytes, String mimeType})?>();

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

    testWidgets('shows error icon when fetchAudio returns null', (
      tester,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: AudioPlayerWidget(fetchAudio: () async => null)),
        ),
      );

      await tester.tap(find.byType(IconButton));
      await tester.runAsync(() => Future<void>.delayed(Duration.zero));
      await tester.pump();

      expect(
        find.byIcon(Icons.error_outline),
        findsOneWidget,
        reason: 'null audio — widget shows error icon for retry',
      );
      expect(find.byType(CircularProgressIndicator), findsNothing);
      expect(find.byIcon(Icons.play_circle_outlined), findsNothing);
    });

    testWidgets('shows error icon when fetchAudio throws', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AudioPlayerWidget(
              fetchAudio: () async => throw Exception('network error'),
            ),
          ),
        ),
      );

      await tester.tap(find.byType(IconButton));
      await tester.runAsync(() => Future<void>.delayed(Duration.zero));
      await tester.pump();

      expect(
        find.byIcon(Icons.error_outline),
        findsOneWidget,
        reason: 'exception during fetch — widget shows error icon',
      );
      expect(find.byType(CircularProgressIndicator), findsNothing);
    });

    testWidgets('tapping error icon retries the fetch', (tester) async {
      int callCount = 0;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AudioPlayerWidget(
              fetchAudio: () async {
                callCount++;
                if (callCount == 1) return null; // first call fails
                return (
                  bytes: Uint8List.fromList([1, 2, 3]),
                  mimeType: 'audio/mpeg',
                ); // retry succeeds
              },
            ),
          ),
        ),
      );

      // First tap → null → error state.
      await tester.tap(find.byType(IconButton));
      await tester.runAsync(() => Future<void>.delayed(Duration.zero));
      await tester.pump();
      expect(find.byIcon(Icons.error_outline), findsOneWidget);

      // Tap the error icon to retry.
      await tester.tap(find.byKey(const Key('audio_error_retry')));
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 50)),
      );
      await tester.pump();

      // After successful retry, should be playing (stop icon visible).
      expect(
        find.byIcon(Icons.stop_circle_outlined),
        findsOneWidget,
        reason: 'retry succeeded — widget should be playing',
      );
      expect(callCount, equals(2));
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
                return (
                  bytes: Uint8List.fromList([1, 2, 3]),
                  mimeType: 'audio/mpeg',
                );
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
    /// Wraps a VoiceRecorderButton in the required ProviderScope.
    Widget wrapWithProviders(Widget child) {
      return ProviderScope(
        overrides: [isOnlineProvider.overrideWithValue(true)],
        child: MaterialApp(home: Scaffold(body: child)),
      );
    }

    testWidgets('initially shows the mic icon and no stop icon', (
      tester,
    ) async {
      await tester.pumpWidget(
        wrapWithProviders(
          VoiceRecorderButton(
            onRecordingComplete: (bytes, mime) {},
            onError: (_) {},
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
        wrapWithProviders(
          VoiceRecorderButton(
            onRecordingComplete: (bytes, mime) {},
            onError: (e) => capturedError = e,
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

    testWidgets('shows stop button and timer once recording starts', (
      tester,
    ) async {
      // Pre-create a stub file so stop() has something to return.
      final stubFile = File(
        '${Directory.systemTemp.path}/voice_stub_${DateTime.now().millisecondsSinceEpoch}.m4a',
      );
      await tester.runAsync(() => stubFile.writeAsBytes([1, 2, 3]));

      _setPathProviderMock();

      await tester.pumpWidget(
        wrapWithProviders(
          VoiceRecorderButton(
            onRecordingComplete: (_, _) {},
            onError: (_) {},
            audioRecorder: _FakeAudioRecorder(stubFile.path),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.mic_none_outlined));
      // Let channel futures and setState calls resolve.
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 100)),
      );
      await tester.pump();

      expect(
        find.byIcon(Icons.stop_circle_outlined),
        findsOneWidget,
        reason: 'stop button must appear once recording is active',
      );
      expect(find.byIcon(Icons.mic_none_outlined), findsNothing);
      // Timer text format: M:SS (initially "2:00" for 120 s max).
      expect(find.text('2:00'), findsOneWidget);

      // Clean up
      if (stubFile.existsSync()) await tester.runAsync(() => stubFile.delete());
    });

    testWidgets('calls onRecordingComplete with bytes and MIME after stop', (
      tester,
    ) async {
      final expectedBytes = Uint8List.fromList([0xDE, 0xAD, 0xBE, 0xEF]);
      final audioFile = File(
        '${Directory.systemTemp.path}/voice_audio_${DateTime.now().millisecondsSinceEpoch}.m4a',
      );
      await tester.runAsync(() => audioFile.writeAsBytes(expectedBytes));

      _setPathProviderMock();

      Uint8List? capturedBytes;
      String? capturedMime;

      await tester.pumpWidget(
        wrapWithProviders(
          VoiceRecorderButton(
            onRecordingComplete: (bytes, mime) {
              capturedBytes = bytes;
              capturedMime = mime;
            },
            onError: (_) {},
            audioRecorder: _FakeAudioRecorder(audioFile.path),
          ),
        ),
      );

      // Start recording.
      await tester.tap(find.byIcon(Icons.mic_none_outlined));
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 100)),
      );
      await tester.pump();

      expect(find.byIcon(Icons.stop_circle_outlined), findsOneWidget);

      // Stop recording — tap and await inside runAsync so the channel call
      // and subsequent file I/O both resolve in real-async context.
      await tester.runAsync(() async {
        await tester.tap(find.byIcon(Icons.stop_circle_outlined));
        await Future<void>.delayed(const Duration(milliseconds: 150));
      });
      await tester.pump();

      expect(
        capturedBytes,
        equals(expectedBytes),
        reason: 'callback must receive the bytes written by the recorder',
      );
      // On native the encoder is aacLc → MPEG-4 container → audio/mp4.
      expect(capturedMime, equals('audio/mp4'));
      // After stop the mic button returns.
      expect(find.byIcon(Icons.mic_none_outlined), findsOneWidget);

      // Clean up (readRecordedOutput deletes the file, but guard anyway).
      if (audioFile.existsSync()) {
        await tester.runAsync(() => audioFile.delete());
      }
    });
  });
}
