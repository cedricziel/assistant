import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:record/record.dart';

import 'package:assistant_app/api/connectivity_provider.dart';
import 'package:assistant_app/features/chat/voice_recorder_button.dart';

/// Fake recorder that tracks whether start() was called.
class _FakeAudioRecorder implements AudioRecorder {
  bool startCalled = false;

  @override
  Future<bool> hasPermission({bool request = true}) async => true;

  @override
  Future<void> start(RecordConfig config, {required String path}) async {
    startCalled = true;
  }

  @override
  Future<void> dispose() async {}

  @override
  Future<bool> isEncoderSupported(AudioEncoder encoder) async => true;

  @override
  Future<String?> stop() async => null;

  @override
  dynamic noSuchMethod(Invocation invocation) {
    // Return appropriate futures for methods that return them.
    final memberName = invocation.memberName.toString();
    if (memberName.contains('cancel') || memberName.contains('pause')) {
      return Future<void>.value();
    }
    return null;
  }
}

void main() {
  group('VoiceRecorderButton offline guard', () {
    testWidgets('does not start recording when offline and shows SnackBar', (
      tester,
    ) async {
      final recorder = _FakeAudioRecorder();
      var completedCalled = false;
      String? errorMsg;

      await tester.pumpWidget(
        ProviderScope(
          overrides: [isOnlineProvider.overrideWithValue(false)],
          child: MaterialApp(
            home: Scaffold(
              body: VoiceRecorderButton(
                audioRecorder: recorder,
                onRecordingComplete: (Uint8List bytes, String mime) {
                  completedCalled = true;
                },
                onError: (String error) {
                  errorMsg = error;
                },
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Tap the mic button.
      await tester.tap(find.byIcon(Icons.mic_none_outlined));
      await tester.pumpAndSettle();

      // Recording should NOT have started.
      expect(recorder.startCalled, isFalse);
      expect(completedCalled, isFalse);
      expect(errorMsg, isNull);

      // SnackBar should appear with the offline message.
      expect(
        find.text('Voice recording requires an internet connection'),
        findsOneWidget,
      );
    });

    testWidgets('does not show offline snackbar when online', (tester) async {
      final recorder = _FakeAudioRecorder();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [isOnlineProvider.overrideWithValue(true)],
          child: MaterialApp(
            home: Scaffold(
              body: VoiceRecorderButton(
                audioRecorder: recorder,
                onRecordingComplete: (_, _) {},
                onError: (_) {},
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Tap the mic button.
      await tester.tap(find.byIcon(Icons.mic_none_outlined));
      await tester.pumpAndSettle();

      // The offline guard snackbar should NOT appear.
      expect(
        find.text('Voice recording requires an internet connection'),
        findsNothing,
      );
    });
  });
}
