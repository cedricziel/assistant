@Tags(['golden'])
library;

import 'dart:typed_data';

import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_input_row.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/conversation_list.dart';
import 'package:assistant_app/features/chat/empty_chat.dart';
import 'package:assistant_app/features/chat/turn_progress_card.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../support/golden.dart';

// Before-image of ChatInputRow, TurnProgressCard, ConversationList and
// EmptyChat (chat-element-library PR 1b).

// -- ChatInputRow ----------------------------------------------------------

const _inputSize = Size(420, 160);

Future<void> _pumpInput(
  WidgetTester tester, {
  bool isSending = false,
  int queued = 0,
  List<PendingAttachment> attachments = const [],
  List<CommandEntry> commands = const [],
  String text = '',
  Size size = _inputSize,
  Alignment alignment = Alignment.bottomCenter,
  Brightness brightness = Brightness.light,
}) async {
  final controller = TextEditingController();
  final focusNode = FocusNode();
  addTearDown(controller.dispose);
  addTearDown(focusNode.dispose);

  await pumpGolden(
    tester,
    ChatInputRow(
      controller: controller,
      focusNode: focusNode,
      isSending: isSending,
      pendingQueueCount: queued,
      pendingAttachments: attachments,
      capabilities: ServerCapabilities.disabled,
      commands: commands,
      onSend: () {},
      onStop: () {},
      onVoiceRecorded: (_, _) {},
      onPickImage: () {},
      onRemoveAttachment: (_) {},
      onPasteImage: (_) {},
    ),
    size: size,
    alignment: alignment,
    brightness: brightness,
  );
  if (text.isNotEmpty) controller.text = text;
  await tester.pump(const Duration(milliseconds: 100));
}

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

// -- TurnProgressCard ------------------------------------------------------

class _FixedChatNotifier extends ChatNotifier {
  _FixedChatNotifier(this._state);

  final ChatState _state;

  @override
  Future<ChatState> build() async => _state;
}

Future<void> _pumpCard(WidgetTester tester, TurnStatusSnapshot snapshot) async {
  await pumpGolden(
    tester,
    const TurnProgressCard(),
    size: const Size(420, 90),
    overrides: [
      chatProvider.overrideWith(
        () => _FixedChatNotifier(ChatState(currentTurnStatus: snapshot)),
      ),
    ],
  );
  // Let the AsyncNotifier build resolve, then render the card.
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 100));
}

// -- ConversationList ------------------------------------------------------

class _FixedListNotifier extends ConversationListNotifier {
  _FixedListNotifier(this._state);

  final ConversationListState _state;

  @override
  Future<ConversationListState> build() async => _state;
}

Future<void> _pumpList(
  WidgetTester tester,
  List<ConversationListEntry> entries,
) async {
  await pumpGolden(
    tester,
    const SizedBox(width: 320, height: 420, child: ConversationList()),
    size: const Size(360, 460),
    overrides: [
      conversationListProvider.overrideWith(
        () => _FixedListNotifier(ConversationListState(conversations: entries)),
      ),
    ],
  );
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 100));
}

ConversationListEntry _entry(String id, String title, Duration ago) {
  final at = goldenAgo(ago);
  return ConversationListEntry(
    id: id,
    title: title,
    createdAt: at,
    updatedAt: at,
  );
}

void main() {
  group('ChatInputRow', () {
    testWidgets('idle', (tester) async {
      await _pumpInput(tester);
      await expectGolden('input_idle');
    });

    testWidgets('idle dark', (tester) async {
      await _pumpInput(tester, brightness: Brightness.dark);
      await expectGolden('input_idle_dark');
    });

    testWidgets('sending', (tester) async {
      await _pumpInput(tester, isSending: true);
      await expectGolden('input_sending');
    });

    testWidgets('queue badge', (tester) async {
      await _pumpInput(tester, queued: 2);
      await expectGolden('input_queue_badge');
    });

    testWidgets('attachments', (tester) async {
      final png = (await tester.runAsync(() => solidPng(Colors.teal)))!;
      await _pumpInput(
        tester,
        size: const Size(420, 240),
        attachments: [
          PendingAttachment(
            bytes: png,
            filename: 'photo.png',
            mimeType: 'image/png',
          ),
          PendingAttachment(
            bytes: Uint8List(8),
            filename: 'quarterly-report.pdf',
            mimeType: 'application/pdf',
          ),
        ],
      );
      await expectGolden('input_attachments');
    });

    testWidgets('slash menu open', (tester) async {
      await _pumpInput(
        tester,
        size: const Size(420, 340),
        commands: _commands,
        text: '/',
      );
      await expectGolden('input_slash_menu');
    });
  });

  group('TurnProgressCard', () {
    testWidgets('active', (tester) async {
      await _pumpCard(
        tester,
        TurnStatusSnapshot(
          runId: 'run-1',
          lastEventKind: TurnEventKind.toolResult,
          lastEventAt: goldenAgo(const Duration(seconds: 2)),
          currentToolName: 'web-search',
        ),
      );
      await expectGolden('turn_progress_active');
    });

    testWidgets('stalled with skip', (tester) async {
      await _pumpCard(
        tester,
        TurnStatusSnapshot(
          runId: 'run-1',
          lastEventKind: TurnEventKind.thinking,
          // 1:35 elapsed; mid-second so the label is stable.
          lastEventAt: goldenAgo(
            const Duration(seconds: 95, milliseconds: 500),
          ),
        ),
      );
      await expectGolden('turn_progress_stalled');
    });
  });

  group('ConversationList', () {
    testWidgets('empty', (tester) async {
      await _pumpList(tester, const []);
      await expectGolden('conversation_list_empty');
    });

    testWidgets('several items', (tester) async {
      await _pumpList(tester, [
        _entry('1', 'Plan the launch', const Duration(seconds: 20)),
        _entry(
          '2',
          'Debug flaky CI job',
          const Duration(minutes: 5, seconds: 30),
        ),
        _entry(
          '3',
          'A very long conversation title that has to be truncated',
          const Duration(hours: 2, minutes: 30),
        ),
        _entry(
          '4',
          'Summarise meeting notes',
          const Duration(days: 3, hours: 12),
        ),
      ]);
      await expectGolden('conversation_list_items');
    });
  });

  group('EmptyChat', () {
    testWidgets('light', (tester) async {
      await pumpGolden(tester, const EmptyChat(), size: const Size(420, 300));
      await tester.pump(const Duration(milliseconds: 100));
      await expectGolden('empty_chat');
    });

    testWidgets('dark', (tester) async {
      await pumpGolden(
        tester,
        const EmptyChat(),
        size: const Size(420, 300),
        brightness: Brightness.dark,
      );
      await tester.pump(const Duration(milliseconds: 100));
      await expectGolden('empty_chat_dark');
    });
  });
}
