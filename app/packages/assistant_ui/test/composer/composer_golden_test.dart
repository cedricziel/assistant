@Tags(['golden'])
library;

import 'dart:typed_data';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

const _commands = [
  CommandEntry(
    name: 'help',
    description: 'Show available commands',
    category: 'General',
  ),
  CommandEntry(
    name: 'model',
    description: 'Switch the active model',
    category: 'Session',
    args: [CommandArg(name: 'name', required: true)],
  ),
  CommandEntry(
    name: 'clear',
    description: 'Clear the conversation',
    category: 'Session',
  ),
];

void main() {
  final cases =
      <
        String,
        ({
          bool isSending,
          bool isStopping,
          int queued,
          String text,
          List<CommandEntry> commands,
          Size size,
        })
      >{
        'composer_idle': (
          isSending: false,
          isStopping: false,
          queued: 0,
          text: '',
          commands: const [],
          size: const Size(420, 100),
        ),
        'composer_sending': (
          isSending: true,
          isStopping: false,
          queued: 0,
          text: '',
          commands: const [],
          size: const Size(420, 100),
        ),
        'composer_stopping': (
          isSending: true,
          isStopping: true,
          queued: 0,
          text: '',
          commands: const [],
          size: const Size(420, 100),
        ),
        'composer_queue_badge': (
          isSending: false,
          isStopping: false,
          queued: 2,
          text: '',
          commands: const [],
          size: const Size(420, 130),
        ),
        'composer_slash_menu': (
          isSending: false,
          isStopping: false,
          queued: 0,
          text: '/',
          commands: _commands,
          size: const Size(420, 260),
        ),
      };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        final c = entry.value;
        final controller = TextEditingController();
        final focusNode = FocusNode();
        addTearDown(controller.dispose);
        addTearDown(focusNode.dispose);

        await pumpGolden(
          tester,
          Composer(
            controller: controller,
            focusNode: focusNode,
            isSending: c.isSending,
            isStopping: c.isStopping,
            pendingQueueCount: c.queued,
            pendingAttachments: const [],
            commands: c.commands,
            onSend: () {},
            onStop: () {},
            onPickImage: () {},
            onRemoveAttachment: (_) {},
          ),
          size: c.size,
          brightness: brightness,
        );
        if (c.text.isNotEmpty) controller.text = c.text;
        await tester.pump(const Duration(milliseconds: 50));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }

  testWidgets('composer_attachments (light)', (tester) async {
    final controller = TextEditingController();
    final focusNode = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focusNode.dispose);

    await pumpGolden(
      tester,
      Composer(
        controller: controller,
        focusNode: focusNode,
        isSending: false,
        pendingQueueCount: 0,
        pendingAttachments: [
          PendingAttachment(
            bytes: Uint8List(8),
            filename: 'quarterly-report.pdf',
            mimeType: 'application/pdf',
          ),
          PendingAttachment(
            bytes: Uint8List(8),
            filename: 'data.csv',
            mimeType: 'text/csv',
          ),
        ],
        onSend: () {},
        onStop: () {},
        onPickImage: () {},
        onRemoveAttachment: (_) {},
      ),
      size: const Size(420, 180),
    );
    await tester.pump(const Duration(milliseconds: 50));

    await expectLater(
      find.byType(MaterialApp),
      matchesGoldenFile('goldens/composer_attachments_light.png'),
    );
  });

  for (final brightness in Brightness.values) {
    testWidgets('slash_command_menu (${brightness.name})', (tester) async {
      await pumpGolden(
        tester,
        Padding(
          padding: const EdgeInsets.all(12),
          child: SlashCommandMenu(
            commands: _commands,
            filter: '',
            onSelect: (_) {},
          ),
        ),
        size: const Size(420, 200),
        brightness: brightness,
      );
      await tester.pump(const Duration(milliseconds: 50));

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/slash_command_menu_${brightness.name}.png'),
      );
    });
  }
}
