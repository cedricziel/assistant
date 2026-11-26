import 'dart:async';

import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_app/api/models/server_profile.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/personas/persona_detail_provider.dart';
import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:assistant_app/shared/platform/adaptive_app.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../../support/fake_personas_api.dart';

class _FakeChatNotifier extends ChatNotifier {
  final List<String> sent = [];

  @override
  Future<ChatState> build() async => const ChatState();

  @override
  Future<void> sendMessage(
    String message, {
    List<String> attachmentIds = const [],
    List<ChatAttachment> attachments = const [],
  }) async => sent.add(message);
}

final _alice = PersonaSummary(
  (b) => b
    ..id = 'p1'
    ..name = 'Alice'
    ..description = '',
);

class _ActivePersonasNotifier extends PersonasNotifier {
  @override
  Future<PersonasState> build() async =>
      PersonasState(personas: [_alice], activePersona: _alice);
}

class _LoadingDetailNotifier extends PersonaDetailNotifier {
  _LoadingDetailNotifier() : super('p1');

  @override
  Future<PersonaDetailState> build() => Completer<PersonaDetailState>().future;
}

class _ErrorDetailNotifier extends PersonaDetailNotifier {
  _ErrorDetailNotifier() : super('p1');

  @override
  Future<PersonaDetailState> build() async =>
      const PersonaDetailState(error: 'boom');
}

class _FakeConversationListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => const ConversationListState();

  @override
  Future<void> refresh() async {}
}

Future<_FakeChatNotifier> _pump(
  WidgetTester tester, {
  List<String> starters = const [],
  PersonaDetailNotifier Function()? detail,
}) async {
  final chat = _FakeChatNotifier();
  tester.view.physicalSize = const Size(480, 960);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        chatProvider.overrideWith(() => chat),
        personasProvider.overrideWith(_ActivePersonasNotifier.new),
        conversationListProvider.overrideWith(
          _FakeConversationListNotifier.new,
        ),
        activeProfileProvider.overrideWithValue(
          const ServerProfile(baseUrl: 'http://localhost', token: 'test-token'),
        ),
        apiClientProvider.overrideWithValue(
          FakePersonasApiClient(FakePersonasApi(starters: starters)),
        ),
        if (detail != null) personaDetailProvider('p1').overrideWith(detail),
      ],
      child: MaterialApp(
        home: const ChatScreen(),
        builder: (_, child) => LegacyDesignBridge(child: child!),
      ),
    ),
  );
  // Not pumpAndSettle: the loading case never settles its future.
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 100));
  return chat;
}

void main() {
  group('Chat empty state — Persona starter prompts', () {
    testWidgets('shows starters and a tap sends the prompt', (tester) async {
      final chat = await _pump(tester, starters: ['Plan my week', 'Triage']);

      expect(find.byKey(const Key('starter_prompt_0')), findsOneWidget);
      expect(find.byKey(const Key('starter_prompt_1')), findsOneWidget);
      expect(find.text('Start a conversation'), findsOneWidget);

      await tester.tap(find.byKey(const Key('starter_prompt_0')));
      await tester.pump();

      expect(chat.sent, ['Plan my week']);
    });

    testWidgets('no starters keeps only the default copy', (tester) async {
      await _pump(tester);

      expect(find.text('Start a conversation'), findsOneWidget);
      expect(find.byKey(const Key('starter_prompt_0')), findsNothing);
    });

    testWidgets('while loading only the default copy renders', (tester) async {
      await _pump(tester, detail: _LoadingDetailNotifier.new);

      expect(find.text('Start a conversation'), findsOneWidget);
      expect(find.byKey(const Key('starter_prompt_0')), findsNothing);
    });

    testWidgets('after an error only the default copy renders', (tester) async {
      await _pump(tester, detail: _ErrorDetailNotifier.new);

      expect(find.text('Start a conversation'), findsOneWidget);
      expect(find.byKey(const Key('starter_prompt_0')), findsNothing);
    });
  });
}
