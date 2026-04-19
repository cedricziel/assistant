import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';
import 'package:assistant_app/features/chat/commands_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// -- Fake notifiers -----------------------------------------------------------

class _FakeChatNotifier extends ChatNotifier {
  @override
  Future<ChatState> build() async => const ChatState();

  void push(ChatState s) => state = AsyncData(s);
}

class _FakePersonasNotifier extends PersonasNotifier {
  @override
  Future<PersonasState> build() async => const PersonasState();
}

class _FakeConversationListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => const ConversationListState();

  @override
  Future<void> refresh() async {}

  @override
  void prependConversation(ConversationSummary conv) {}
}

// -- Harness ------------------------------------------------------------------

/// Pumps [ChatScreen] with faked providers including a commands list.
Future<({_FakeChatNotifier notifier})> pumpWithCommands(
  WidgetTester tester, {
  List<CommandEntry> commands = const [],
}) async {
  final chatNotifier = _FakeChatNotifier();

  tester.view.physicalSize = const Size(480, 960);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);

  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        chatProvider.overrideWith(() => chatNotifier),
        personasProvider.overrideWith(() => _FakePersonasNotifier()),
        conversationListProvider.overrideWith(
          () => _FakeConversationListNotifier(),
        ),
        apiClientProvider.overrideWithValue(null),
        commandsProvider.overrideWith((ref) async => commands),
      ],
      child: const MaterialApp(home: ChatScreen()),
    ),
  );

  await tester.pumpAndSettle();
  return (notifier: chatNotifier);
}

// -- Sample commands ----------------------------------------------------------

final _sampleCommands = [
  const CommandEntry(name: 'help', description: 'Show help', category: 'info'),
  const CommandEntry(
    name: 'new',
    description: 'New session',
    category: 'session',
  ),
  const CommandEntry(
    name: 'model',
    description: 'Set model',
    category: 'config',
    args: [CommandArg(name: 'model_name', required: false)],
  ),
  const CommandEntry(
    name: 'stop',
    description: 'Stop turn',
    category: 'session',
  ),
  const CommandEntry(
    name: 'compact',
    description: 'Compact history',
    category: 'session',
  ),
  const CommandEntry(
    name: 'status',
    description: 'Show status',
    category: 'info',
  ),
];

// -- Tests --------------------------------------------------------------------

void main() {
  group('Command autocomplete popup', () {
    testWidgets('typing / in empty input shows autocomplete popup', (
      tester,
    ) async {
      await pumpWithCommands(tester, commands: _sampleCommands);

      // Type / in the input field.
      final input = find.byKey(const Key('message_input'));
      await tester.enterText(input, '/');
      await tester.pump(const Duration(milliseconds: 400));

      expect(
        find.byKey(const Key('command_autocomplete_popup')),
        findsOneWidget,
        reason: 'Autocomplete popup should appear when / is typed',
      );
    });

    testWidgets('popup shows all commands when only / is typed', (
      tester,
    ) async {
      await pumpWithCommands(tester, commands: _sampleCommands);

      final input = find.byKey(const Key('message_input'));
      await tester.enterText(input, '/');
      await tester.pump(const Duration(milliseconds: 400));

      // All 6 commands should be listed.
      for (final cmd in _sampleCommands) {
        expect(
          find.text('/${cmd.name}'),
          findsOneWidget,
          reason: '/${cmd.name} should appear in the popup',
        );
      }
    });

    testWidgets('popup filters commands by prefix as user types', (
      tester,
    ) async {
      await pumpWithCommands(tester, commands: _sampleCommands);

      final input = find.byKey(const Key('message_input'));
      await tester.enterText(input, '/mo');
      await tester.pump(const Duration(milliseconds: 400));

      // Only /model should match.
      expect(find.text('/model'), findsOneWidget);
      expect(find.text('/help'), findsNothing);
      expect(find.text('/new'), findsNothing);
    });

    testWidgets('popup hides when input no longer starts with /', (
      tester,
    ) async {
      await pumpWithCommands(tester, commands: _sampleCommands);

      final input = find.byKey(const Key('message_input'));

      // Show popup.
      await tester.enterText(input, '/');
      await tester.pump(const Duration(milliseconds: 400));
      expect(
        find.byKey(const Key('command_autocomplete_popup')),
        findsOneWidget,
      );

      // Clear the field — popup should disappear.
      await tester.enterText(input, '');
      await tester.pump(const Duration(milliseconds: 400));
      expect(
        find.byKey(const Key('command_autocomplete_popup')),
        findsNothing,
        reason: 'Popup should hide when / is removed',
      );
    });

    testWidgets('tapping a no-arg command submits immediately', (tester) async {
      await pumpWithCommands(tester, commands: _sampleCommands);

      final input = find.byKey(const Key('message_input'));
      await tester.enterText(input, '/');
      await tester.pump(const Duration(milliseconds: 400));

      // Tap /new (no args).
      await tester.tap(find.text('/new'));
      await tester.pump(const Duration(milliseconds: 400));

      // Popup should dismiss.
      expect(
        find.byKey(const Key('command_autocomplete_popup')),
        findsNothing,
        reason: 'Popup should dismiss after selecting a no-arg command',
      );
    });

    testWidgets(
      'tapping a command with args fills input instead of submitting',
      (tester) async {
        await pumpWithCommands(tester, commands: _sampleCommands);

        final input = find.byKey(const Key('message_input'));
        await tester.enterText(input, '/');
        await tester.pump();

        // Tap /model (has args).
        await tester.tap(find.text('/model'));
        await tester.pump(const Duration(milliseconds: 400));

        // Input should be filled with "/model " and popup should dismiss.
        final controller = tester.widget<TextField>(
          find.byKey(const Key('message_input')),
        );
        expect(
          controller.controller?.text,
          '/model ',
          reason: 'Selecting /model should fill input with "/model "',
        );
      },
    );

    testWidgets('popup not shown when / is mid-text', (tester) async {
      await pumpWithCommands(tester, commands: _sampleCommands);

      final input = find.byKey(const Key('message_input'));
      await tester.enterText(input, 'hello /');
      await tester.pump(const Duration(milliseconds: 400));

      expect(
        find.byKey(const Key('command_autocomplete_popup')),
        findsNothing,
        reason: 'Popup should not appear when / is not the first character',
      );
    });
  });
}
