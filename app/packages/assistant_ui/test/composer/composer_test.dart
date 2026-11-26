// Widget tests for [Composer]. Pins:
//   - send and stop emit callbacks; stopping disables stop
//   - the queue-depth badge
//   - the injected input and the neutral default input
//   - the slash menu and the voice slot
//
// Specified by: openspec/changes/chat-element-library/specs/
//   chat-element-library/spec.md ("Composer emits intent by callback and
//   performs no I/O").

import 'dart:typed_data';
import 'dart:ui' show Tristate;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

const _commands = [
  CommandEntry(name: 'help', description: 'Show help', category: 'info'),
  CommandEntry(name: 'new', description: 'New session', category: 'session'),
  CommandEntry(
    name: 'model',
    description: 'Set model',
    category: 'config',
    args: [CommandArg(name: 'model_name', required: false)],
  ),
];

class _Harness {
  _Harness() : controller = TextEditingController(), focusNode = FocusNode() {
    addTearDown(controller.dispose);
    addTearDown(focusNode.dispose);
  }

  final TextEditingController controller;
  final FocusNode focusNode;
  int sends = 0;
  int stops = 0;
  int picks = 0;
  final removed = <int>[];

  Future<void> pump(
    WidgetTester tester, {
    bool isSending = false,
    bool isStopping = false,
    int queued = 0,
    List<PendingAttachment> attachments = const [],
    List<CommandEntry> commands = const [],
    bool showVoice = false,
    Widget? voiceButton,
    ComposerInputBuilder? inputBuilder,
  }) async {
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
              pendingQueueCount: queued,
              pendingAttachments: attachments,
              commands: commands,
              showVoice: showVoice,
              voiceButton: voiceButton,
              inputBuilder: inputBuilder,
              onSend: () => sends++,
              onStop: () => stops++,
              onPickImage: () => picks++,
              onRemoveAttachment: removed.add,
            ),
          ),
        ),
      ),
    );
  }
}

void main() {
  group('Composer', () {
    testWidgets('send fires onSend', (tester) async {
      final h = _Harness();
      await h.pump(tester);

      await tester.tap(find.byKey(const Key('send_button')));

      expect(h.sends, 1);
      expect(find.byKey(const Key('stop_button')), findsNothing);
    });

    testWidgets('submitting the default input fires onSend', (tester) async {
      final h = _Harness();
      await h.pump(tester);

      await tester.enterText(find.byKey(const Key('message_input')), 'hi');
      await tester.testTextInput.receiveAction(TextInputAction.send);

      expect(h.sends, 1);
    });

    testWidgets('isSending shows stop_button, which fires onStop', (
      tester,
    ) async {
      final h = _Harness();
      await h.pump(tester, isSending: true);

      expect(find.byKey(const Key('send_button')), findsNothing);
      await tester.tap(find.byKey(const Key('stop_button')));

      expect(h.stops, 1);
      expect(h.sends, 0);
    });

    testWidgets('isStopping disables stop with a "Stopping…" label', (
      tester,
    ) async {
      final h = _Harness();
      await h.pump(tester, isSending: true, isStopping: true);

      await tester.tap(find.byKey(const Key('stop_button')));

      expect(h.stops, 0);
      final node = tester.getSemantics(find.byKey(const Key('stop_button')));
      // The tooltip is the accessible name Flutter exposes for the button.
      expect(node.tooltip, 'Stopping…');
      expect(
        node.getSemanticsData().flagsCollection.isEnabled,
        Tristate.isFalse,
      );
    });

    testWidgets('the queue badge shows the count', (tester) async {
      final h = _Harness();
      await h.pump(tester, queued: 1);
      expect(find.text('1 message queued'), findsOneWidget);

      await h.pump(tester, queued: 3);
      expect(find.text('3 messages queued'), findsOneWidget);
    });

    testWidgets('no queue badge when nothing is queued', (tester) async {
      final h = _Harness();
      await h.pump(tester);
      expect(find.byKey(const Key('queue_depth_badge')), findsNothing);
    });

    testWidgets('inputBuilder replaces the default input', (tester) async {
      final h = _Harness();
      TextEditingController? seenController;
      FocusNode? seenFocus;
      String? seenPlaceholder;
      await h.pump(
        tester,
        inputBuilder: (context, controller, focusNode, placeholder, onSubmit) {
          seenController = controller;
          seenFocus = focusNode;
          seenPlaceholder = placeholder;
          return TextButton(
            key: const Key('custom_input'),
            onPressed: onSubmit,
            child: const Text('custom'),
          );
        },
      );

      expect(find.byKey(const Key('custom_input')), findsOneWidget);
      expect(find.byType(TextField), findsNothing);
      expect(seenController, same(h.controller));
      expect(seenFocus, same(h.focusNode));
      expect(seenPlaceholder, 'Type a message...');

      await tester.tap(find.byKey(const Key('custom_input')));
      expect(h.sends, 1);
    });

    testWidgets('the placeholder asks for a caption with attachments', (
      tester,
    ) async {
      final h = _Harness();
      String? seenPlaceholder;
      await h.pump(
        tester,
        attachments: [
          PendingAttachment(
            bytes: Uint8List(1),
            filename: 'a.pdf',
            mimeType: 'application/pdf',
          ),
        ],
        inputBuilder: (context, controller, focusNode, placeholder, onSubmit) {
          seenPlaceholder = placeholder;
          return const SizedBox();
        },
      );

      expect(seenPlaceholder, 'Add a caption...');
    });

    testWidgets('the default input renders without the façade', (tester) async {
      final h = _Harness();
      await h.pump(tester);

      expect(find.byKey(const Key('message_input')), findsOneWidget);
      expect(find.byType(TextField), findsOneWidget);
    });

    testWidgets('typing / shows the slash menu', (tester) async {
      final h = _Harness();
      await h.pump(tester, commands: _commands);

      await tester.enterText(find.byKey(const Key('message_input')), '/');
      await tester.pump();

      expect(find.byType(SlashCommandMenu), findsOneWidget);
      expect(find.text('/help'), findsOneWidget);
    });

    testWidgets('no slash menu without commands', (tester) async {
      final h = _Harness();
      await h.pump(tester);

      await tester.enterText(find.byKey(const Key('message_input')), '/');
      await tester.pump();

      expect(find.byType(SlashCommandMenu), findsNothing);
    });

    testWidgets('selecting a no-arg command fires onSend', (tester) async {
      final h = _Harness();
      await h.pump(tester, commands: _commands);

      await tester.enterText(find.byKey(const Key('message_input')), '/');
      await tester.pump();
      await tester.tap(find.text('/new'));
      await tester.pump();

      expect(h.controller.text, '/new');
      expect(h.sends, 1);
      expect(find.byType(SlashCommandMenu), findsNothing);
    });

    testWidgets('selecting a command with args fills the input', (
      tester,
    ) async {
      final h = _Harness();
      await h.pump(tester, commands: _commands);

      await tester.enterText(find.byKey(const Key('message_input')), '/');
      await tester.pump();
      await tester.tap(find.text('/model'));
      await tester.pump();

      expect(h.controller.text, '/model ');
      expect(h.sends, 0);
    });

    testWidgets('the voice slot renders when showVoice and idle', (
      tester,
    ) async {
      final h = _Harness();
      const voice = SizedBox(key: Key('voice_slot'), width: 24, height: 24);
      await h.pump(tester, showVoice: true, voiceButton: voice);
      expect(find.byKey(const Key('voice_slot')), findsOneWidget);

      await h.pump(tester, showVoice: false, voiceButton: voice);
      expect(find.byKey(const Key('voice_slot')), findsNothing);

      await h.pump(
        tester,
        showVoice: true,
        isSending: true,
        voiceButton: voice,
      );
      expect(find.byKey(const Key('voice_slot')), findsNothing);
    });

    testWidgets('attach fires onPickImage and hides while sending', (
      tester,
    ) async {
      final h = _Harness();
      await h.pump(tester);
      await tester.tap(find.byKey(const Key('attach_image_button')));
      expect(h.picks, 1);

      await h.pump(tester, isSending: true);
      expect(find.byKey(const Key('attach_image_button')), findsNothing);
    });

    testWidgets('removing an attachment is delegated', (tester) async {
      final h = _Harness();
      await h.pump(
        tester,
        attachments: [
          PendingAttachment(
            bytes: Uint8List(1),
            filename: 'a.pdf',
            mimeType: 'application/pdf',
          ),
          PendingAttachment(
            bytes: Uint8List(1),
            filename: 'b.pdf',
            mimeType: 'application/pdf',
          ),
        ],
      );

      await tester.tap(find.byKey(const Key('remove_attachment_0')));

      expect(h.removed, [0]);
    });
  });

  group('SlashCommandMenu', () {
    Future<List<CommandEntry>> pumpMenu(
      WidgetTester tester, {
      required String filter,
    }) async {
      final selected = <CommandEntry>[];
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SlashCommandMenu(
              commands: _commands,
              filter: filter,
              onSelect: selected.add,
            ),
          ),
        ),
      );
      return selected;
    }

    testWidgets('an empty filter lists every command', (tester) async {
      await pumpMenu(tester, filter: '');
      for (final c in _commands) {
        expect(find.text('/${c.name}'), findsOneWidget);
      }
    });

    testWidgets('the filter matches by prefix', (tester) async {
      await pumpMenu(tester, filter: 'mo');
      expect(find.text('/model'), findsOneWidget);
      expect(find.text('/help'), findsNothing);
    });

    testWidgets('no match renders nothing', (tester) async {
      await pumpMenu(tester, filter: 'zzz');
      expect(find.byType(InkWell), findsNothing);
    });

    testWidgets('selecting fires onSelect without dispatching', (tester) async {
      final selected = await pumpMenu(tester, filter: '');
      await tester.tap(find.text('/new'));
      expect(selected.map((c) => c.name), ['new']);
    });
  });
}
