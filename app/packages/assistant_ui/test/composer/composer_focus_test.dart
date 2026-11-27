// Keyboard and focus tests for [Composer].
//
// Specified by: openspec/changes/chat-a11y-and-code-blocks/specs/
//   chat-accessibility/spec.md ("Composer focus follows visual order and is
//   preserved").

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

const _commands = [
  CommandEntry(name: 'help', description: 'Show help', category: 'info'),
  CommandEntry(name: 'new', description: 'New session', category: 'session'),
];

class _Harness {
  _Harness() : controller = TextEditingController(), inputFocus = FocusNode() {
    addTearDown(controller.dispose);
    addTearDown(inputFocus.dispose);
  }

  final TextEditingController controller;
  final FocusNode inputFocus;
  int sends = 0;
  int stops = 0;

  Future<void> pump(
    WidgetTester tester, {
    bool isSending = false,
    bool isStopping = false,
    List<CommandEntry> commands = const [],
    bool showVoice = false,
  }) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.bottomCenter,
            child: Composer(
              controller: controller,
              focusNode: inputFocus,
              isSending: isSending,
              isStopping: isStopping,
              pendingQueueCount: 0,
              pendingAttachments: const [],
              commands: commands,
              showVoice: showVoice,
              voiceButton: showVoice
                  ? ComposerVoiceButton(
                      isRecording: false,
                      onStart: () {},
                      onStop: () {},
                    )
                  : null,
              onSend: () => sends++,
              onStop: () => stops++,
              onPickImage: () {},
              onRemoveAttachment: (_) {},
            ),
          ),
        ),
      ),
    );
  }
}

/// The control that currently owns focus, found by walking up from the
/// primary focus: its key, `voice` for the voice affordance, or null.
String? _focusedControl() {
  final context = FocusManager.instance.primaryFocus?.context;
  if (context == null) return null;
  String? found;
  context.visitAncestorElements((element) {
    final widget = element.widget;
    if (widget is ComposerVoiceButton) {
      found = 'voice';
      return false;
    }
    final key = widget.key;
    if ((widget is IconButton || widget is TextField) && key != null) {
      found = key is ValueKey<String> ? key.value : key.toString();
      return false;
    }
    return true;
  });
  return found;
}

Future<void> _focusInput(WidgetTester tester, _Harness harness) async {
  harness.inputFocus.requestFocus();
  await tester.pump();
}

Future<void> _tab(WidgetTester tester) async {
  await tester.sendKeyEvent(LogicalKeyboardKey.tab);
  await tester.pump();
}

void main() {
  group('Composer focus order', () {
    testWidgets('Tab follows attach, voice, input, send, then wraps', (
      tester,
    ) async {
      final h = _Harness();
      await h.pump(tester, showVoice: true);
      await _focusInput(tester, h);

      expect(_focusedControl(), 'message_input');

      await _tab(tester);
      expect(_focusedControl(), 'send_button');

      await _tab(tester);
      expect(_focusedControl(), 'attach_image_button');

      await _tab(tester);
      expect(_focusedControl(), 'voice');

      await _tab(tester);
      expect(
        _focusedControl(),
        'message_input',
        reason: 'the traversal cycle holds only the four visible controls',
      );
    });

    testWidgets('the traversal cycle holds no invisible focus node', (
      tester,
    ) async {
      final h = _Harness();
      await h.pump(tester, showVoice: true);
      await _focusInput(tester, h);

      final visited = <String?>[];
      for (var i = 0; i < 4; i++) {
        await _tab(tester);
        visited.add(_focusedControl());
      }

      expect(visited, [
        'send_button',
        'attach_image_button',
        'voice',
        'message_input',
      ]);
    });
  });

  group('Composer focus hand-over', () {
    testWidgets('Enter submits and leaves focus in the input', (tester) async {
      final h = _Harness();
      await h.pump(tester);

      await tester.enterText(find.byKey(const Key('message_input')), 'hi');
      await tester.testTextInput.receiveAction(TextInputAction.send);
      await tester.pump();

      expect(h.sends, 1);
      expect(h.inputFocus.hasFocus, isTrue);
      expect(_focusedControl(), 'message_input');
    });

    testWidgets('send to stop moves focus to the stop button', (tester) async {
      final h = _Harness();
      await h.pump(tester);

      final send = tester
          .widget<IconButton>(find.byKey(const Key('send_button')))
          .focusNode;
      expect(send, isNotNull, reason: 'send owns the composer focus node');
      send!.requestFocus();
      await tester.pump();
      expect(_focusedControl(), 'send_button');

      await tester.tap(find.byKey(const Key('send_button')));
      await tester.pump();
      expect(h.sends, 1);

      await h.pump(tester, isSending: true);
      await tester.pump();

      expect(_focusedControl(), 'stop_button');
    });

    testWidgets('a tap on send leaves focus in the input', (tester) async {
      final h = _Harness();
      await h.pump(tester);
      await _focusInput(tester, h);

      await tester.tap(find.byKey(const Key('send_button')));
      await tester.pump();

      expect(h.sends, 1);
      expect(_focusedControl(), 'message_input');
    });

    testWidgets('stop moves focus to the input before it is disabled', (
      tester,
    ) async {
      final h = _Harness();
      await h.pump(tester, isSending: true);

      await tester.tap(find.byKey(const Key('stop_button')));
      expect(h.stops, 1);
      await tester.pump();

      // Activation alone hands focus over; no rebuild is needed for it.
      expect(_focusedControl(), 'message_input');

      await h.pump(tester, isSending: true, isStopping: true);
      await tester.pump();
      expect(
        tester
            .widget<IconButton>(find.byKey(const Key('stop_button')))
            .onPressed,
        isNull,
        reason: 'stop is disabled while stopping',
      );
      expect(_focusedControl(), 'message_input');

      await h.pump(tester);
      await tester.pump();
      expect(_focusedControl(), 'message_input');
    });
  });

  group('Composer keyboard shortcuts', () {
    testWidgets('Escape closes the slash menu and keeps input and text', (
      tester,
    ) async {
      final h = _Harness();
      await h.pump(tester, commands: _commands);

      await tester.enterText(find.byKey(const Key('message_input')), '/');
      await tester.pump();
      expect(find.byType(SlashCommandMenu), findsOneWidget);
      expect(h.inputFocus.hasFocus, isTrue);

      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pump();

      expect(find.byType(SlashCommandMenu), findsNothing);
      expect(h.inputFocus.hasFocus, isTrue);
      expect(h.controller.text, '/');
    });

    testWidgets('Escape without an open menu changes nothing', (tester) async {
      final h = _Harness();
      await h.pump(tester, commands: _commands);

      await tester.enterText(find.byKey(const Key('message_input')), 'hello');
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pump();

      expect(h.controller.text, 'hello');
      expect(h.inputFocus.hasFocus, isTrue);
      expect(h.sends, 0);
    });
  });
}
