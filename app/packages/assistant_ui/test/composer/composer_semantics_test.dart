// Semantics and tap-target tests for [Composer] and its voice button.
//
// Specified by: openspec/changes/chat-a11y-and-code-blocks/specs/
//   chat-accessibility/spec.md ("Icon-only controls have labels and adequate
//   targets").

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Future<void> _pump(
  WidgetTester tester, {
  bool isSending = false,
  bool isStopping = false,
  bool showVoice = false,
  bool isRecording = false,
  List<PendingAttachment> attachments = const [],
}) async {
  final controller = TextEditingController();
  final focusNode = FocusNode();
  addTearDown(controller.dispose);
  addTearDown(focusNode.dispose);
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Align(
          alignment: Alignment.bottomCenter,
          child: Composer(
            controller: controller,
            focusNode: focusNode,
            isSending: isSending,
            isStopping: isStopping,
            pendingQueueCount: 0,
            pendingAttachments: attachments,
            showVoice: showVoice,
            voiceButton: ComposerVoiceButton(
              isRecording: isRecording,
              onStart: () {},
              onStop: () {},
            ),
            onSend: () {},
            onStop: () {},
            onPickImage: () {},
            onRemoveAttachment: (_) {},
          ),
        ),
      ),
    ),
  );
}

/// The accessible name Flutter exposes: the label, or the tooltip an
/// [IconButton] contributes in its place.
String _name(WidgetTester tester, Finder finder) {
  final node = tester.getSemantics(finder);
  return node.label.isNotEmpty ? node.label : node.tooltip;
}

void main() {
  group('Composer semantics', () {
    testWidgets('send button is labelled "Send message"', (tester) async {
      final handle = tester.ensureSemantics();
      await _pump(tester);

      expect(
        _name(tester, find.byKey(const Key('send_button'))),
        'Send message',
      );
      handle.dispose();
    });

    testWidgets('stop button is labelled "Stop response"', (tester) async {
      final handle = tester.ensureSemantics();
      await _pump(tester, isSending: true);

      expect(
        _name(tester, find.byKey(const Key('stop_button'))),
        'Stop response',
      );
      handle.dispose();
    });

    testWidgets('stop button reads "Stopping…" while stopping', (tester) async {
      final handle = tester.ensureSemantics();
      await _pump(tester, isSending: true, isStopping: true);

      expect(_name(tester, find.byKey(const Key('stop_button'))), 'Stopping…');
      handle.dispose();
    });

    testWidgets('attach button is labelled "Attach image"', (tester) async {
      final handle = tester.ensureSemantics();
      await _pump(tester);

      expect(
        _name(tester, find.byKey(const Key('attach_image_button'))),
        'Attach image',
      );
      handle.dispose();
    });

    testWidgets(
      'voice button reads "Send voice message" then "Stop recording"',
      (tester) async {
        final handle = tester.ensureSemantics();
        await _pump(tester, showVoice: true);
        final button = find.descendant(
          of: find.byType(ComposerVoiceButton),
          matching: find.byType(IconButton),
        );
        expect(_name(tester, button), 'Send voice message');

        await _pump(tester, showVoice: true, isRecording: true);
        expect(_name(tester, button), 'Stop recording');
        handle.dispose();
      },
    );

    testWidgets('send and stop carry tooltips', (tester) async {
      await _pump(tester);
      expect(
        tester.widget<IconButton>(find.byKey(const Key('send_button'))).tooltip,
        'Send message',
      );
      await _pump(tester, isSending: true);
      expect(
        tester.widget<IconButton>(find.byKey(const Key('stop_button'))).tooltip,
        'Stop response',
      );
    });
  });

  group('Composer tap targets', () {
    testWidgets('meet the 48 dp Android guideline with voice and no '
        'attachments', (tester) async {
      final handle = tester.ensureSemantics();
      await _pump(tester, showVoice: true);

      await expectLater(tester, meetsGuideline(androidTapTargetGuideline));
      handle.dispose();
    });

    testWidgets('are labelled', (tester) async {
      final handle = tester.ensureSemantics();
      await _pump(tester, showVoice: true);

      await expectLater(tester, meetsGuideline(labeledTapTargetGuideline));
      handle.dispose();
    });
  });
}
