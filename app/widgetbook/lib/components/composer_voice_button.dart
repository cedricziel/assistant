import 'package:assistant_ui/assistant_ui.dart';
import 'package:widgetbook/widgetbook.dart';

final composerVoiceButtonComponent = WidgetbookComponent(
  name: 'ComposerVoiceButton',
  useCases: [
    WidgetbookUseCase(
      name: 'idle',
      builder: (context) => ComposerVoiceButton(
        isRecording: false,
        onStart: () {},
        onStop: () {},
      ),
    ),
    WidgetbookUseCase(
      name: 'recording',
      builder: (context) => ComposerVoiceButton(
        isRecording: true,
        elapsed: const Duration(seconds: 42),
        onStart: () {},
        onStop: () {},
      ),
    ),
  ],
);
