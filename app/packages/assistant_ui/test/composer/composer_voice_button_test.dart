import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Widget _host(Widget child) => MaterialApp(home: Scaffold(body: child));

void main() {
  testWidgets('idle shows the mic and fires onStart', (tester) async {
    var started = 0;
    await tester.pumpWidget(
      _host(
        ComposerVoiceButton(
          isRecording: false,
          onStart: () => started++,
          onStop: () {},
        ),
      ),
    );

    expect(find.byIcon(Icons.mic_none_outlined), findsOneWidget);
    expect(find.byIcon(Icons.stop_circle_outlined), findsNothing);

    await tester.tap(find.byIcon(Icons.mic_none_outlined));
    expect(started, 1);
  });

  testWidgets('recording shows the time left and a stop button', (
    tester,
  ) async {
    var stopped = 0;
    await tester.pumpWidget(
      _host(
        ComposerVoiceButton(
          isRecording: true,
          elapsed: const Duration(seconds: 5),
          onStart: () {},
          onStop: () => stopped++,
        ),
      ),
    );

    expect(find.byIcon(Icons.mic_none_outlined), findsNothing);
    expect(
      find.text('1:55'),
      findsOneWidget,
      reason: 'the label counts down from the default two minute limit',
    );

    await tester.tap(find.byIcon(Icons.stop_circle_outlined));
    expect(stopped, 1);
  });

  testWidgets('maxDuration sets the countdown start', (tester) async {
    await tester.pumpWidget(
      _host(
        ComposerVoiceButton(
          isRecording: true,
          maxDuration: const Duration(seconds: 30),
          onStart: () {},
          onStop: () {},
        ),
      ),
    );

    expect(find.text('0:30'), findsOneWidget);
  });
}
