import 'dart:typed_data';

import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/message_actions_builder.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../support/fake_audioplayers.dart';

const _voice = ServerCapabilities(voiceSend: false, voiceReceive: true);

Widget _wrap(
  ChatMessage message, {
  ServerCapabilities capabilities = ServerCapabilities.disabled,
  VoidCallback? onRetry,
  VoidCallback? onStop,
  bool isStopping = false,
  Widget? timing,
  Widget? feedback,
  bool isPersisted = false,
  bool turnWorking = false,
  VoidCallback? onEdit,
  VoidCallback? onRegenerate,
  ValueChanged<String>? onRegenerateWith,
  List<RegenerateModel> regenerateModels = const [],
  ValueChanged<String>? onSwitchBranch,
}) => MaterialApp(
  home: Scaffold(
    body: Align(
      alignment: Alignment.topLeft,
      child: buildMessageActions(
        message: message,
        capabilities: capabilities,
        fetchMessageAudio: () async => (bytes: Uint8List(0), mimeType: ''),
        onRetry: onRetry,
        onStop: onStop,
        isStopping: isStopping,
        timing: timing,
        feedback: feedback,
        isPersisted: isPersisted,
        turnWorking: turnWorking,
        onEdit: onEdit,
        onRegenerate: onRegenerate,
        onRegenerateWith: onRegenerateWith,
        regenerateModels: regenerateModels,
        onSwitchBranch: onSwitchBranch,
      ),
    ),
  ),
);

ChatMessage _user({List<ChatAttachment>? attachments, BranchInfo? branch}) =>
    ChatMessage(
      id: 'u',
      role: 'user',
      content: 'A question',
      attachments: attachments,
      branch: branch,
    );

const _branch = BranchInfo(count: 3, index: 1, siblingIds: ['m1', 'm2', 'm3']);

const _models = [
  RegenerateModel(name: 'model-a', isDefault: true),
  RegenerateModel(name: 'model-b'),
];

ChatMessage _assistant({
  String id = 'a',
  String content = 'A reply',
  BranchInfo? branch,
  MessageStatus status = MessageStatus.ok,
  bool isStreaming = false,
  TimelineEntryType timelineType = TimelineEntryType.message,
}) => ChatMessage(
  id: id,
  role: 'assistant',
  content: content,
  status: status,
  isStreaming: isStreaming,
  branch: branch,
  timelineType: timelineType,
);

double _left(WidgetTester tester, String key) =>
    tester.getTopLeft(find.byKey(Key(key))).dx;

void _ignoreRating(MessageRating? _) {}

void main() {
  setUp(installFakeAudioplayers);

  group('buildMessageActions', () {
    testWidgets('failed assistant message lists actions in today\'s order', (
      tester,
    ) async {
      await tester.pumpWidget(
        _wrap(
          _assistant(status: MessageStatus.failed),
          capabilities: _voice,
          onRetry: () {},
        ),
      );

      const keys = [
        'read_aloud_action',
        'copy_action',
        'share_action',
        'save_action',
        'retry_button',
      ];
      for (final k in keys) {
        expect(find.byKey(Key(k)), findsOneWidget, reason: k);
      }
      final lefts = [for (final k in keys) _left(tester, k)];
      expect(lefts, [...lefts]..sort(), reason: 'left-to-right order');
      expect(find.byKey(const Key('stop_action')), findsNothing);
    });

    testWidgets('retry uses the danger tone', (tester) async {
      await tester.pumpWidget(
        _wrap(_assistant(status: MessageStatus.failed), onRetry: () {}),
      );

      final retry = tester.widget<MessageActionButton>(
        find.byKey(const Key('retry_button')),
      );
      expect(retry.tone, MessageActionTone.danger);
      final copy = tester.widget<MessageActionButton>(
        find.byKey(const Key('copy_action')),
      );
      expect(copy.tone, MessageActionTone.muted);
    });

    testWidgets('user messages only get Copy', (tester) async {
      await tester.pumpWidget(
        _wrap(ChatMessage(id: 'u', role: 'user', content: 'hi')),
      );

      expect(find.byKey(const Key('copy_action')), findsOneWidget);
      expect(find.byKey(const Key('share_action')), findsNothing);
      expect(find.byKey(const Key('save_action')), findsNothing);
      expect(find.byKey(const Key('read_aloud_action')), findsNothing);
    });

    testWidgets('a streaming message shows Stop only', (tester) async {
      await tester.pumpWidget(
        _wrap(_assistant(isStreaming: true), onStop: () {}),
      );

      expect(find.byKey(const Key('stop_action')), findsOneWidget);
      expect(find.byKey(const Key('copy_action')), findsNothing);
    });

    testWidgets('a stopping message shows a disabled Stopping… action', (
      tester,
    ) async {
      var stops = 0;
      await tester.pumpWidget(
        _wrap(
          _assistant(isStreaming: true),
          onStop: () => stops++,
          isStopping: true,
        ),
      );

      expect(find.text('Stopping…'), findsOneWidget);
      final stop = tester.widget<MessageActionButton>(
        find.byKey(const Key('stop_action')),
      );
      expect(stop.onPressed, isNull);
      await tester.tap(find.byKey(const Key('stop_action')));
      expect(stops, 0);
    });

    testWidgets('a streaming message without onStop renders nothing', (
      tester,
    ) async {
      await tester.pumpWidget(_wrap(_assistant(isStreaming: true)));

      expect(find.byType(MessageActionButton), findsNothing);
    });

    testWidgets('an empty user message renders nothing', (tester) async {
      await tester.pumpWidget(
        _wrap(ChatMessage(id: 'u', role: 'user', content: '')),
      );

      expect(tester.getSize(find.byType(MessageActions)), Size.zero);
    });

    testWidgets('trailing is timing first, then feedback, after the actions', (
      tester,
    ) async {
      await tester.pumpWidget(
        _wrap(
          _assistant(),
          timing: const Text('timing', key: Key('timing')),
          feedback: const Text('feedback', key: Key('feedback')),
        ),
      );

      expect(_left(tester, 'save_action'), lessThan(_left(tester, 'timing')));
      expect(_left(tester, 'timing'), lessThan(_left(tester, 'feedback')));
    });

    testWidgets('a stopped assistant message shows Stopped and Copy', (
      tester,
    ) async {
      await tester.pumpWidget(
        _wrap(
          ChatMessage(
            id: 'a1',
            role: 'assistant',
            content: 'The capital of France',
            status: MessageStatus.stopped,
          ),
        ),
      );

      expect(find.byKey(const Key('stopped_label')), findsOneWidget);
      expect(find.text('Stopped'), findsOneWidget);
      expect(find.byKey(const Key('copy_action')), findsOneWidget);
      expect(find.byKey(const Key('stop_action')), findsNothing);
    });

    testWidgets('an ordinary assistant message shows no Stopped label', (
      tester,
    ) async {
      await tester.pumpWidget(_wrap(_assistant(content: 'done')));

      expect(find.byKey(const Key('stopped_label')), findsNothing);
      expect(find.byKey(const Key('copy_action')), findsOneWidget);
    });
  });

  group('buildMessageTrailing', () {
    List<Widget> trailing(
      ChatMessage message, {
      Widget? timing,
      ValueChanged<MessageRating?>? onRatingChanged = _ignoreRating,
    }) => buildMessageTrailing(
      message: message,
      timing: timing,
      rating: message.feedback?.rating,
      onRatingChanged: onRatingChanged,
    );

    test('an assistant reply gets feedback buttons after any timing', () {
      const timing = Text('timing', key: Key('timing'));
      final widgets = trailing(_assistant(), timing: timing);

      expect(widgets.first, same(timing));
      expect(widgets.last, isA<FeedbackButtons>());
      expect(widgets, hasLength(2));
    });

    test('the current rating reaches the buttons', () {
      final message = _assistant()
        ..feedback = const MessageFeedback(rating: MessageRating.positive);

      final buttons = trailing(message).single as FeedbackButtons;

      expect(buttons.rating, MessageRating.positive);
      expect(buttons.enabled, isTrue);
    });

    test('taps are reported through onRatingChanged', () {
      MessageRating? last;
      final widgets = trailing(
        _assistant(),
        onRatingChanged: (rating) => last = rating,
      );
      final buttons = widgets.single as FeedbackButtons;

      buttons.onChanged(MessageRating.negative);

      expect(last, MessageRating.negative);
    });

    test('user messages get no buttons', () {
      expect(trailing(_user()), isEmpty);
    });

    test('streaming replies get no buttons', () {
      expect(trailing(_assistant(isStreaming: true)), isEmpty);
    });

    test('failed replies get no buttons', () {
      expect(trailing(_assistant(status: MessageStatus.failed)), isEmpty);
    });

    test('empty or blank replies get no buttons', () {
      expect(trailing(_assistant(content: '')), isEmpty);
      expect(trailing(_assistant(content: '   ')), isEmpty);
    });

    test('non-message timeline entries get no buttons', () {
      expect(
        trailing(_assistant(timelineType: TimelineEntryType.thinking)),
        isEmpty,
      );
      expect(
        trailing(_assistant(timelineType: TimelineEntryType.toolCall)),
        isEmpty,
      );
    });

    test('the unsaved assistant- fallback id gets no buttons', () {
      expect(trailing(_assistant(id: 'assistant-1759238400000')), isEmpty);
    });

    test('without a callback there is nothing to wire', () {
      expect(trailing(_assistant(), onRatingChanged: null), isEmpty);
    });
  });

  group('buildMessageActions branch entries', () {
    final edit = find.byKey(const Key('edit_action'));
    final regenerate = find.byKey(const Key('regenerate_action'));

    testWidgets('a persisted user message offers Edit', (tester) async {
      var edited = 0;
      await tester.pumpWidget(
        _wrap(_user(), isPersisted: true, onEdit: () => edited++),
      );

      expect(edit, findsOneWidget);
      await tester.tap(edit);
      expect(edited, 1);
      expect(regenerate, findsNothing);
    });

    testWidgets('Edit is offered for a user message with attachments', (
      tester,
    ) async {
      await tester.pumpWidget(
        _wrap(
          _user(
            attachments: const [
              ChatAttachment(
                id: 'f',
                filename: 'a.png',
                mimeType: 'image/png',
                url: '/a.png',
              ),
            ],
          ),
          isPersisted: true,
          onEdit: () {},
        ),
      );

      expect(edit, findsOneWidget);
    });

    testWidgets('an unpersisted user message offers no Edit', (tester) async {
      await tester.pumpWidget(_wrap(_user(), onEdit: () {}));

      expect(edit, findsNothing);
    });

    testWidgets('a persisted assistant message offers Regenerate, not Edit', (
      tester,
    ) async {
      var regenerated = 0;
      await tester.pumpWidget(
        _wrap(
          _assistant(),
          isPersisted: true,
          onEdit: () {},
          onRegenerate: () => regenerated++,
        ),
      );

      expect(regenerate, findsOneWidget);
      await tester.tap(regenerate);
      expect(regenerated, 1);
      expect(edit, findsNothing);
    });

    testWidgets('a user message offers no Regenerate', (tester) async {
      await tester.pumpWidget(
        _wrap(_user(), isPersisted: true, onRegenerate: () {}),
      );

      expect(regenerate, findsNothing);
    });

    testWidgets('no callbacks, no entries', (tester) async {
      await tester.pumpWidget(_wrap(_user(), isPersisted: true));
      expect(edit, findsNothing);

      await tester.pumpWidget(_wrap(_assistant(), isPersisted: true));
      expect(regenerate, findsNothing);
    });

    testWidgets('both are withheld while a turn works', (tester) async {
      await tester.pumpWidget(
        _wrap(_user(), isPersisted: true, turnWorking: true, onEdit: () {}),
      );
      expect(edit, findsNothing);

      await tester.pumpWidget(
        _wrap(
          _assistant(),
          isPersisted: true,
          turnWorking: true,
          onRegenerate: () {},
        ),
      );
      expect(regenerate, findsNothing);
    });

    testWidgets('both are offered when no turn works, as while awaitingInput', (
      tester,
    ) async {
      // `turnWorking` is false both when idle and when the turn only waits
      // for the user, so the same call covers both states.
      await tester.pumpWidget(
        _wrap(
          _assistant(),
          isPersisted: true,
          turnWorking: false,
          onRegenerate: () {},
        ),
      );

      expect(regenerate, findsOneWidget);
    });

    testWidgets('leading is a BranchPicker when the message has a branch', (
      tester,
    ) async {
      final switched = <String>[];
      await tester.pumpWidget(
        _wrap(
          _assistant(branch: _branch),
          isPersisted: true,
          onSwitchBranch: switched.add,
        ),
      );

      final picker = tester.widget<BranchPicker>(find.byType(BranchPicker));
      expect((picker.index, picker.count, picker.busy), (1, 3, false));
      expect(
        tester.widget<MessageActions>(find.byType(MessageActions)).leading,
        isA<BranchPicker>(),
      );
      picker.onPrevious();
      picker.onNext();
      expect(switched, ['m1', 'm3']);
    });

    testWidgets('the picker is busy only while a turn works', (tester) async {
      await tester.pumpWidget(
        _wrap(
          _user(
            branch: const BranchInfo(
              count: 2,
              index: 0,
              siblingIds: ['a', 'b'],
            ),
          ),
          isPersisted: true,
          turnWorking: true,
          onSwitchBranch: (_) {},
        ),
      );

      expect(
        tester.widget<BranchPicker>(find.byType(BranchPicker)).busy,
        isTrue,
      );
    });

    testWidgets('no picker without a branch', (tester) async {
      await tester.pumpWidget(
        _wrap(_assistant(), isPersisted: true, onSwitchBranch: (_) {}),
      );

      expect(find.byType(BranchPicker), findsNothing);
    });

    testWidgets('the regenerate menu needs the callback and two models', (
      tester,
    ) async {
      final picked = <String>[];
      await tester.pumpWidget(
        _wrap(
          _assistant(),
          isPersisted: true,
          onRegenerateWith: picked.add,
          regenerateModels: _models,
        ),
      );
      expect(find.byType(RegenerateMenuButton), findsOneWidget);

      await tester.pumpWidget(
        _wrap(
          _assistant(),
          isPersisted: true,
          onRegenerateWith: picked.add,
          regenerateModels: _models.take(1).toList(),
        ),
      );
      expect(find.byType(PopupMenuButton<String>), findsNothing);

      await tester.pumpWidget(
        _wrap(_assistant(), isPersisted: true, regenerateModels: _models),
      );
      expect(find.byType(RegenerateMenuButton), findsNothing);

      await tester.pumpWidget(
        _wrap(
          _user(),
          isPersisted: true,
          onRegenerateWith: picked.add,
          regenerateModels: _models,
        ),
      );
      expect(find.byType(RegenerateMenuButton), findsNothing);

      await tester.pumpWidget(
        _wrap(
          _assistant(),
          isPersisted: true,
          turnWorking: true,
          onRegenerateWith: picked.add,
          regenerateModels: _models,
        ),
      );
      expect(find.byType(RegenerateMenuButton), findsNothing);
    });
  });
}
