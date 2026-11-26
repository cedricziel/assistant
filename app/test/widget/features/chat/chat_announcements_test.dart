import 'dart:async';

import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/capabilities_provider.dart';
import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/api/models/server_profile.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:assistant_app/shared/platform/adaptive_app.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

class _StreamingApiClient extends ApiClient {
  _StreamingApiClient(this.controller)
    : super(baseUrl: 'http://fake', token: 'fake');

  final StreamController<StreamEvent> controller;

  @override
  Stream<StreamEvent> streamMessages(
    String conversationId,
    String message, {
    List<String>? attachmentIds,
  }) => controller.stream;
}

class _AdaPersonasNotifier extends PersonasNotifier {
  @override
  Future<PersonasState> build() async => PersonasState(
    activePersona: PersonaSummary(
      (b) => b
        ..id = 'ada'
        ..name = 'Ada'
        ..description = '',
    ),
  );
}

class _FakeConversationListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => const ConversationListState();

  @override
  Future<void> refresh() async {}

  @override
  void prependConversation(ConversationSummary conv) {}
}

void main() {
  testWidgets('a scripted turn announces once, and streaming is not live', (
    tester,
  ) async {
    final announced = <String>[];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockDecodedMessageHandler<Object?>(SystemChannels.accessibility, (
          message,
        ) async {
          final map = message! as Map<Object?, Object?>;
          if (map['type'] == 'announce') {
            announced.add((map['data']! as Map)['message']! as String);
          }
          return null;
        });
    addTearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockDecodedMessageHandler<Object?>(
            SystemChannels.accessibility,
            null,
          ),
    );
    final semantics = tester.ensureSemantics();

    tester.view.physicalSize = const Size(480, 960);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final ctrl = StreamController<StreamEvent>();
    addTearDown(() async {
      if (!ctrl.isClosed) unawaited(ctrl.close());
    });
    late ChatNotifier notifier;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          apiClientProvider.overrideWithValue(_StreamingApiClient(ctrl)),
          personasProvider.overrideWith(() => _AdaPersonasNotifier()),
          conversationListProvider.overrideWith(
            () => _FakeConversationListNotifier(),
          ),
          activeProfileProvider.overrideWithValue(
            const ServerProfile(baseUrl: 'http://fake', token: 'fake'),
          ),
          capabilitiesProvider.overrideWith(
            (ref) async =>
                const ServerCapabilities(voiceSend: false, voiceReceive: false),
          ),
        ],
        child: MaterialApp(
          home: Consumer(
            builder: (context, ref, _) {
              notifier = ref.watch(chatProvider.notifier);
              return const ChatScreen();
            },
          ),
          builder: (_, child) => LegacyDesignBridge(child: child!),
        ),
      ),
    );
    await tester.pumpAndSettle();
    notifier.setConversationId('conv-1');

    unawaited(notifier.sendMessage('go'));
    await tester.pump();
    await tester.runAsync(() async {
      ctrl.add(const RunStartedEvent('run-1'));
      await Future<void>.delayed(Duration.zero);
      for (var i = 0; i < 20; i++) {
        ctrl.add(TokenEvent('word$i '));
        await Future<void>.delayed(Duration.zero);
      }
    });
    await tester.pump();

    // Mid-stream, no node is a live region, so TalkBack does not re-read the
    // bubble per token.
    expect(notifier.state.value!.streamingContent, contains('word19'));
    var liveRegions = 0;
    void walk(SemanticsNode node) {
      if (node.getSemanticsData().flagsCollection.isLiveRegion) liveRegions++;
      node.visitChildren((child) {
        walk(child);
        return true;
      });
    }

    walk(tester.getSemantics(find.byType(ChatScreen)));
    expect(liveRegions, 0, reason: 'the streaming bubble is not a live region');

    await tester.runAsync(() async {
      ctrl.add(const DoneEvent(role: 'assistant', content: '**Hello** there'));
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);
    });
    await tester.pump();
    await tester.pump(const Duration(seconds: 2));
    await tester.pump(const Duration(seconds: 2));

    expect(announced, ['Ada replied: Hello there']);
    unawaited(ctrl.close());
    await tester.pumpAndSettle();
    semantics.dispose();
  });
}
