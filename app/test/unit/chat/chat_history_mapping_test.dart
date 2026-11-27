import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/chat/chat_provider.dart' hide BranchInfo;
import 'package:assistant_app/features/chat/chat_turn_transitions.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:flutter_test/flutter_test.dart';

MessageSummary _msg({
  required String id,
  required String role,
  required String content,
  int turn = 0,
  String? finishReason,
  DateTime? createdAt,
  List<({String name, String status, String? result, String? id})> toolCalls =
      const [],
  List<({String id, String filename, String mimeType, String url})>
      attachments =
      const [],
  ({String rating, String? comment})? feedback,
  ({int totalMs, int? ttftMs, int? outputTokens, int? decodeMs})? timing,
}) {
  return MessageSummary(
    (b) => b
      ..id = id
      ..role = role
      ..content = content
      ..turn = turn
      ..createdAt = createdAt ?? DateTime.parse('2026-05-16T00:00:00Z')
      ..ttsAvailable = false
      ..feedback = feedback == null
          ? null
          : (MessageFeedbackSummaryBuilder()
              ..rating = feedback.rating
              ..comment = feedback.comment
              ..updatedAt = DateTime.parse('2026-05-16T00:00:00Z'))
      ..finishReason = finishReason
      ..timing = timing == null
          ? null
          : (MessageTimingSummaryBuilder()
              ..totalMs = timing.totalMs
              ..timeToFirstTokenMs = timing.ttftMs
              ..outputTokens = timing.outputTokens
              ..decodeMs = timing.decodeMs)
      ..toolCalls = toolCalls.isEmpty
          ? null
          : ListBuilder<ToolCallSummary>(
              toolCalls.map(
                (tc) => ToolCallSummary(
                  (t) => t
                    ..name = tc.name
                    ..status = tc.status
                    ..result = tc.result
                    ..toolCallId = tc.id,
                ),
              ),
            )
      ..attachments = attachments.isEmpty
          ? null
          : ListBuilder<AttachmentMetaResponse>(
              attachments.map(
                (a) => AttachmentMetaResponse(
                  (x) => x
                    ..id = a.id
                    ..filename = a.filename
                    ..mimeType = a.mimeType
                    ..url = a.url
                    ..sizeBytes = 1
                    ..createdAt = DateTime.parse('2026-05-16T00:00:00Z'),
                ),
              ),
            ),
  );
}

InteractionDetail _interaction({
  required String id,
  String kind = 'tool_approval',
  InteractionStatusView status = InteractionStatusView.resolved,
  String? toolCallId,
  String runId = 'run-1',
  DateTime? createdAt,
  Map<String, Object?> request = const {
    'tool_name': 'bash',
    'arguments': {'command': 'git status'},
    'options': ['allow_once', 'allow_always', 'reject_once'],
  },
  Map<String, Object?>? response = const {'decision': 'allow_once'},
}) {
  final created = createdAt ?? DateTime.parse('2026-05-16T00:01:30Z');
  return InteractionDetail(
    (b) => b
      ..interactionId = id
      ..conversationId = 'conv-1'
      ..runId = runId
      ..kind = kind
      ..status = status
      ..rendersCard = true
      ..toolCallId = toolCallId
      ..createdAt = created
      ..expiresAt = created.add(const Duration(minutes: 10))
      ..resolvedAt = status == InteractionStatusView.resolved
          ? created.add(const Duration(seconds: 5))
          : null
      ..request = JsonObject(request)
      ..response = response == null ? null : JsonObject(response),
  );
}

void main() {
  group('chatMessagesFromHistory', () {
    test('carries parentId, branch and replyBranch onto ChatMessage', () {
      final rows = [
        MessageSummary(
          (b) => b
            ..id = 'u2'
            ..role = 'user'
            ..content = 'edited'
            ..turn = 1
            ..createdAt = DateTime.parse('2026-05-16T00:00:00Z')
            ..ttsAvailable = false
            ..parentId = 'a1'
            ..branch.replace(
              BranchInfo(
                (x) => x
                  ..count = 2
                  ..index = 1
                  ..siblingIds.replace(['u2-old', 'u2']),
              ),
            )
            ..replyBranch.replace(
              BranchInfo(
                (x) => x
                  ..count = 3
                  ..index = 2
                  ..siblingIds.replace(['r1', 'r2', 'r3']),
              ),
            ),
        ),
      ];

      final out = chatMessagesFromHistory(rows).single;

      expect(out.parentId, 'a1');
      expect(out.branch?.count, 2);
      expect(out.branch?.index, 1);
      expect(out.branch?.siblingIds, ['u2-old', 'u2']);
      expect(out.replyBranch?.count, 3);
      expect(out.replyBranch?.index, 2);
      expect(out.replyBranch?.siblingIds, ['r1', 'r2', 'r3']);
    });

    test('maps timing onto the row that carries it', () {
      final out = chatMessagesFromHistory([
        _msg(id: 'u1', role: 'user', content: 'Hello'),
        _msg(
          id: 'a1',
          role: 'assistant',
          content: 'Hi',
          timing: (
            totalMs: 3000,
            ttftMs: 800,
            outputTokens: 120,
            decodeMs: 2200,
          ),
        ),
        _msg(id: 'u2', role: 'user', content: 'Again'),
        _msg(id: 'a2', role: 'assistant', content: 'No timing'),
      ]);

      final byId = {for (final m in out) m.id: m};
      final timing = byId['a1']!.timing;
      expect(
        timing?.timeToFirstToken,
        const Duration(milliseconds: 800),
        reason: 'history timing must land on the assistant row that has it',
      );
      expect(timing?.total, const Duration(milliseconds: 3000));
      expect(timing?.outputTokens, 120);
      expect(timing?.decode, const Duration(milliseconds: 2200));
      expect(byId['u1']!.timing, isNull);
      expect(byId['a2']!.timing, isNull);
    });

    test('rows without branch metadata map to nulls', () {
      final out = chatMessagesFromHistory([
        _msg(id: 'm', role: 'user', content: 'hi'),
      ]).single;

      expect(out.parentId, isNull);
      expect(out.branch, isNull);
      expect(out.replyBranch, isNull);
    });

    test('tool-only rows are still skipped when they carry branch data', () {
      final toolOnly = _msg(
        id: 'a2',
        role: 'assistant',
        content: '',
        toolCalls: [(name: 'web-search', status: 'ok', result: 'r', id: null)],
      ).rebuild((b) => b..parentId = 'u1');

      final out = chatMessagesFromHistory([toolOnly]);

      expect(
        out.where((m) => m.timelineType == TimelineEntryType.message),
        isEmpty,
      );
      expect(out.single.timelineType, TimelineEntryType.toolCall);
    });

    test(
      'assistant tool-call-only turns render as chips, not empty bubbles',
      () {
        // Mirrors the actual schorschvm conversation shape: 3 ReAct iterations
        // each persisted as `assistant(content="", tool_calls=[X])` followed
        // by a `tool` result row, terminated by an assistant final answer.
        final history = [
          _msg(id: 'u1', role: 'user', content: 'Hello', turn: 0),
          _msg(
            id: 'a1',
            role: 'assistant',
            content: '',
            turn: 1,
            toolCalls: [(name: 'bash', status: 'ok', result: 'ok', id: null)],
          ),
          _msg(
            id: 'a2',
            role: 'assistant',
            content: '',
            turn: 2,
            toolCalls: [
              (name: 'web-fetch', status: 'ok', result: 'ok', id: null),
            ],
          ),
          _msg(
            id: 'a3',
            role: 'assistant',
            content: '',
            turn: 3,
            toolCalls: [
              (name: 'slack-post', status: 'ok', result: 'ok', id: null),
            ],
          ),
          _msg(id: 'a4', role: 'assistant', content: 'Done.', turn: 4),
        ];

        final mapped = chatMessagesFromHistory(history);

        // For each tool-only assistant row we expect exactly one toolCall
        // timeline entry and *no* accompanying message bubble.
        final toolCallEntries = mapped
            .where((m) => m.timelineType == TimelineEntryType.toolCall)
            .toList();
        expect(toolCallEntries, hasLength(3));
        expect(
          toolCallEntries.map((m) => m.toolCalls.single.toolName).toList(),
          ['bash', 'web-fetch', 'slack-post'],
        );

        // No empty assistant message bubbles. The user row, three chips,
        // and the final-answer row are the only entries we expect.
        final messageBubbles = mapped
            .where((m) => m.timelineType == TimelineEntryType.message)
            .toList();
        expect(messageBubbles, hasLength(2));
        expect(messageBubbles[0].role, 'user');
        expect(messageBubbles[0].content, 'Hello');
        expect(messageBubbles[1].role, 'assistant');
        expect(messageBubbles[1].content, 'Done.');

        for (final m in messageBubbles) {
          expect(
            m.content.isEmpty && m.role == 'assistant',
            isFalse,
            reason: 'Empty assistant content bubble leaked through',
          );
        }
      },
    );

    test(
      'assistant message with both content and tool calls keeps the bubble',
      () {
        final history = [
          _msg(
            id: 'a1',
            role: 'assistant',
            content: 'Calling tool first.',
            toolCalls: [(name: 'bash', status: 'ok', result: 'ok', id: null)],
          ),
        ];

        final mapped = chatMessagesFromHistory(history);

        expect(
          mapped.where((m) => m.timelineType == TimelineEntryType.toolCall),
          hasLength(1),
        );
        final bubbles = mapped
            .where((m) => m.timelineType == TimelineEntryType.message)
            .toList();
        expect(bubbles, hasLength(1));
        expect(bubbles.single.content, 'Calling tool first.');
      },
    );

    test('user messages are always preserved as bubbles', () {
      final history = [
        _msg(id: 'u1', role: 'user', content: 'Hi'),
        _msg(id: 'u2', role: 'user', content: ''),
      ];

      final mapped = chatMessagesFromHistory(history);
      final bubbles = mapped
          .where((m) => m.timelineType == TimelineEntryType.message)
          .toList();
      expect(bubbles, hasLength(2));
      expect(bubbles.every((m) => m.role == 'user'), isTrue);
    });

    test('toolCall chip entries carry name, status, and result', () {
      final history = [
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          toolCalls: [
            (name: 'bash', status: 'ok', result: 'stdout: ok', id: null),
            (name: 'web-fetch', status: 'error', result: '404', id: null),
          ],
        ),
      ];

      final mapped = chatMessagesFromHistory(history);
      final chips = mapped
          .where((m) => m.timelineType == TimelineEntryType.toolCall)
          .toList();
      expect(chips, hasLength(2));
      expect(chips[0].toolCalls.single.toolName, 'bash');
      expect(chips[0].toolCalls.single.status, ToolCallStatus.ok);
      expect(chips[0].toolCalls.single.result, 'stdout: ok');
      expect(chips[1].toolCalls.single.toolName, 'web-fetch');
      expect(chips[1].toolCalls.single.status, ToolCallStatus.error);
      expect(chips[1].toolCalls.single.result, '404');
    });

    test('tool-only assistant row WITH attachments keeps the bubble for thumbnails', () {
      // Edge case: the model invoked a tool AND produced an image attachment
      // but no reply text. The bubble must remain so the attachment renders.
      final history = [
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          toolCalls: [
            (name: 'image-gen', status: 'ok', result: 'ok', id: null),
          ],
          attachments: [
            (
              id: 'att1',
              filename: 'render.png',
              mimeType: 'image/png',
              url: 'https://example/render.png',
            ),
          ],
        ),
      ];

      final mapped = chatMessagesFromHistory(history);
      final bubbles = mapped
          .where((m) => m.timelineType == TimelineEntryType.message)
          .toList();
      expect(bubbles, hasLength(1));
      expect(bubbles.single.role, 'assistant');
      expect(bubbles.single.content, '');
      expect(bubbles.single.attachments, hasLength(1));
      expect(bubbles.single.attachments.single.filename, 'render.png');
      expect(
        mapped.where((m) => m.timelineType == TimelineEntryType.toolCall),
        hasLength(1),
      );
    });

    test('pending tool-call status round-trips (not coerced to ok)', () {
      // A persisted row could carry a non-terminal status if the orchestrator
      // crashed mid-call. The mapper must surface it as `pending` instead of
      // silently treating it as a successful invocation.
      final history = [
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          toolCalls: [
            (name: 'bash', status: 'pending', result: null, id: null),
          ],
        ),
      ];

      final chips = chatMessagesFromHistory(history)
          .where((m) => m.timelineType == TimelineEntryType.toolCall)
          .toList();
      expect(chips, hasLength(1));
      expect(chips.single.toolCalls.single.status, ToolCallStatus.pending);
    });

    test('cancelled tool-call status maps to ToolCallStatus.cancelled', () {
      final history = [
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          toolCalls: [
            (name: 'bash', status: 'cancelled', result: null, id: null),
          ],
        ),
      ];

      final chips = chatMessagesFromHistory(history)
          .where((m) => m.timelineType == TimelineEntryType.toolCall)
          .toList();
      expect(chips.single.toolCalls.single.status, ToolCallStatus.cancelled);
    });

    test('multiple invocations of the same tool produce distinct chip ids', () {
      // Defensive: if a single assistant row carries two `bash` invocations,
      // both chips must have unique ids so Flutter widget keying does not
      // collide.
      final history = [
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          toolCalls: [
            (name: 'bash', status: 'ok', result: 'first', id: null),
            (name: 'bash', status: 'ok', result: 'second', id: null),
          ],
        ),
      ];

      final ids = chatMessagesFromHistory(history)
          .where((m) => m.timelineType == TimelineEntryType.toolCall)
          .map((m) => m.id)
          .toList();
      expect(ids, hasLength(2));
      expect(ids.toSet(), hasLength(2), reason: 'chip ids must be unique');
    });

    test('tool data and tool call id reach the ToolCallRecord', () {
      final history = [
        MessageSummary(
          (b) => b
            ..id = 'a1'
            ..role = 'assistant'
            ..content = ''
            ..turn = 1
            ..createdAt = DateTime.parse('2026-05-16T00:00:00Z')
            ..ttsAvailable = false
            ..toolCalls = ListBuilder<ToolCallSummary>([
              ToolCallSummary(
                (t) => t
                  ..name = 'bash'
                  ..status = 'ok'
                  ..toolCallId = 'call_7'
                  ..data = JsonObject({'kind': 'terminal', 'exit_code': 0}),
              ),
              ToolCallSummary(
                (t) => t
                  ..name = 'bash'
                  ..status = 'ok',
              ),
            ]),
        ),
      ];

      final records = chatMessagesFromHistory(history)
          .map((m) => m.toolCalls.single)
          .toList();

      expect(records[0].toolCallId, 'call_7');
      expect(records[0].data, {'kind': 'terminal', 'exit_code': 0});
      expect(records[1].toolCallId, isNull);
      expect(records[1].data, isNull);
    });

    group('finish reason', () {
      MessageStatus statusOf(String? reason) => chatMessagesFromHistory([
        _msg(id: 'a1', role: 'assistant', content: 'x', finishReason: reason),
      ]).single.status;

      test('cancelled maps to stopped', () {
        expect(statusOf('cancelled'), MessageStatus.stopped);
      });

      test('no finish reason maps to ok', () {
        expect(statusOf(null), MessageStatus.ok);
      });

      test('an unknown finish reason maps to ok', () {
        expect(statusOf('length'), MessageStatus.ok);
      });

      for (final (name, partial, streamed) in [
        ('a partial reply', 'The capital', 'The capital'),
        ('the _Stopped._ placeholder', '_Stopped._', ''),
        ('an empty frame falling back to streamed text', '', 'The capital'),
      ]) {
        test('a live stopped reply and its reload agree for $name', () {
          final live = stoppedTurnState(
            ChatState(
              conversationId: 'c1',
              messages: [
                ChatMessage(id: 'u1', role: 'user', content: 'q'),
                ChatMessage(
                  id: streamingPlaceholderId,
                  role: 'assistant',
                  content: streamed,
                  isStreaming: true,
                ),
              ],
              isSending: true,
              streamingContent: streamed,
            ),
            userMsgId: 'u1',
            partialContent: partial,
            messageId: 'M',
          ).messages.last;

          // The server persists whatever text the frame carried.
          final reloaded = chatMessagesFromHistory([
            _msg(
              id: 'M',
              role: 'assistant',
              content: partial.isNotEmpty ? partial : streamed,
              finishReason: 'cancelled',
            ),
          ]).single;

          expect(reloaded.id, live.id);
          expect(reloaded.status, live.status);
          expect(reloaded.content, live.content);
        });
      }
    });

    test('the MCP App of a tool call reaches the ToolCallRecord', () {
      final history = [
        MessageSummary(
          (b) => b
            ..id = 'a1'
            ..role = 'assistant'
            ..content = ''
            ..turn = 1
            ..createdAt = DateTime.parse('2026-05-16T00:00:00Z')
            ..ttsAvailable = false
            ..toolCalls = ListBuilder<ToolCallSummary>([
              ToolCallSummary(
                (t) => t
                  ..name = 'mcp__weather__forecast'
                  ..status = 'ok'
                  ..toolCallId = 'call_8'
                  ..app.replace(
                    McpAppView(
                      (a) => a
                        ..server = 'weather'
                        ..resourceUri = 'ui://weather/forecast'
                        ..html = '<p>Sunny</p>'
                        ..prefersBorder = true
                        ..csp.connectDomains = ListBuilder([
                          'https://api.example.com',
                        ])
                        ..permissions.camera = false
                        ..permissions.microphone = false
                        ..permissions.geolocation = true
                        ..permissions.clipboardWrite = false,
                    ),
                  ),
              ),
              ToolCallSummary(
                (t) => t
                  ..name = 'bash'
                  ..status = 'ok',
              ),
            ]),
        ),
      ];

      final records = chatMessagesFromHistory(history)
          .map((m) => m.toolCalls.single)
          .toList();

      final app = records[0].app!;
      expect(app.server, 'weather');
      expect(app.resourceUri, 'ui://weather/forecast');
      expect(app.html, '<p>Sunny</p>');
      expect(app.prefersBorder, isTrue);
      expect(app.csp.connectDomains, ['https://api.example.com']);
      expect(app.csp.resourceDomains, isEmpty);
      expect(app.permissions.geolocation, isTrue);
      expect(app.permissions.camera, isFalse);
      expect(records[1].app, isNull);
    });
  });

  group('chatMessagesFromHistory — interactions', () {
    test('a detail with a tool_call_id becomes a receipt on that call', () {
      final history = [
        _msg(id: 'u1', role: 'user', content: 'go', turn: 1),
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          turn: 1,
          toolCalls: [(name: 'bash', status: 'ok', result: 'ok', id: 'call-1')],
        ),
      ];

      final messages = chatMessagesFromHistory(
        history,
        interactions: [_interaction(id: 'int-1', toolCallId: 'call-1')],
      );

      final call = messages
          .firstWhere((m) => m.timelineType == TimelineEntryType.toolCall)
          .toolCalls
          .single;
      expect(call.interaction?.id, 'int-1');
      expect(call.interaction?.status, InteractionStatus.resolved);
      expect(call.interaction?.decision, ApprovalDecision.allowOnce);
      expect(
        messages.where((m) => m.timelineType == TimelineEntryType.interaction),
        isEmpty,
        reason: 'a matched receipt needs no standalone entry',
      );
    });

    test('an interaction whose tool call is not on the active branch renders '
        'nowhere', () {
      final history = [
        _msg(id: 'u1', role: 'user', content: 'go', turn: 1),
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          turn: 1,
          toolCalls: [
            (name: 'bash', status: 'ok', result: 'ok', id: 'call-active'),
          ],
        ),
      ];

      final messages = chatMessagesFromHistory(
        history,
        interactions: [
          _interaction(id: 'int-gone', toolCallId: 'call-regenerated-away'),
        ],
      );

      expect(
        messages.where((m) => m.timelineType == TimelineEntryType.interaction),
        isEmpty,
        reason: 'not parked at the end of an unrelated turn',
      );
      expect(
        messages
            .firstWhere((m) => m.timelineType == TimelineEntryType.toolCall)
            .toolCalls
            .single
            .interaction,
        isNull,
      );
    });

    test('without an id joins per turn and per tool name, then falls back to '
        'the end of the turn', () {
      final history = [
        _msg(
          id: 'u1',
          role: 'user',
          content: 'go',
          turn: 1,
          createdAt: DateTime.parse('2026-05-16T00:01:00Z'),
        ),
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          turn: 1,
          createdAt: DateTime.parse('2026-05-16T00:01:10Z'),
          toolCalls: [
            (name: 'bash', status: 'ok', result: 'A', id: null),
            (name: 'bash', status: 'ok', result: 'B', id: null),
          ],
        ),
        _msg(
          id: 'a2',
          role: 'assistant',
          content: 'done',
          turn: 1,
          createdAt: DateTime.parse('2026-05-16T00:01:40Z'),
        ),
        _msg(
          id: 'u2',
          role: 'user',
          content: 'next',
          turn: 2,
          createdAt: DateTime.parse('2026-05-16T00:02:00Z'),
        ),
      ];

      final messages = chatMessagesFromHistory(
        history,
        interactions: [
          _interaction(
            id: 'int-a',
            createdAt: DateTime.parse('2026-05-16T00:01:20Z'),
          ),
          _interaction(
            id: 'int-b',
            createdAt: DateTime.parse('2026-05-16T00:01:25Z'),
          ),
          _interaction(
            id: 'int-c',
            createdAt: DateTime.parse('2026-05-16T00:01:30Z'),
          ),
        ],
      );

      final calls = messages
          .where((m) => m.timelineType == TimelineEntryType.toolCall)
          .map((m) => m.toolCalls.single)
          .toList();
      expect(calls[0].interaction?.id, 'int-a');
      expect(calls[1].interaction?.id, 'int-b');

      final fallback = messages.singleWhere(
        (m) => m.timelineType == TimelineEntryType.interaction,
      );
      expect(fallback.interaction?.id, 'int-c');
      expect(
        messages.indexOf(fallback),
        greaterThan(messages.indexOf(messages.firstWhere((m) => m.id == 'a2'))),
        reason: 'the fallback sits at the end of its own turn',
      );
      expect(
        messages.indexOf(fallback),
        lessThan(messages.indexOf(messages.firstWhere((m) => m.id == 'u2'))),
        reason: 'never at the end of a later turn',
      );
    });

    test('a question detail carries its questions and answers', () {
      final history = [
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          turn: 1,
          toolCalls: [
            (name: 'ask-user', status: 'ok', result: 'ok', id: 'call-q'),
          ],
        ),
      ];
      final messages = chatMessagesFromHistory(
        history,
        interactions: [
          _interaction(
            id: 'int-q',
            kind: 'question',
            toolCallId: 'call-q',
            request: {
              'questions': [
                {
                  'id': 'q1',
                  'prompt': 'Which branch?',
                  'options': [
                    {'id': 'o1', 'label': 'main'},
                    {'id': 'o2', 'label': 'release'},
                  ],
                  'multiple': false,
                  'free_text': false,
                },
              ],
            },
            response: {
              'answers': [
                {
                  'question_id': 'q1',
                  'selected_option_ids': ['o2'],
                },
              ],
            },
          ),
        ],
      );

      final record = messages
          .firstWhere((m) => m.timelineType == TimelineEntryType.toolCall)
          .toolCalls
          .single
          .interaction!;
      expect(record.kind, InteractionKind.question);
      expect(record.questions.single.prompt, 'Which branch?');
      expect(record.questions.single.options.map((o) => o.label), [
        'main',
        'release',
      ]);
      expect(record.answers.single.selectedOptionIds, ['o2']);
      expect(record.declined, isFalse);
    });

    test('a declined question receipt says so', () {
      final history = [
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          turn: 1,
          toolCalls: [
            (name: 'ask-user', status: 'ok', result: 'ok', id: 'call-q'),
          ],
        ),
      ];
      final record =
          chatMessagesFromHistory(
                history,
                interactions: [
                  _interaction(
                    id: 'int-q',
                    kind: 'question',
                    toolCallId: 'call-q',
                    request: {
                      'questions': [
                        {'id': 'q1', 'prompt': 'Which branch?', 'options': []},
                      ],
                    },
                    response: {'declined': true},
                  ),
                ],
              )
              .firstWhere((m) => m.timelineType == TimelineEntryType.toolCall)
              .toolCalls
              .single
              .interaction!;

      expect(record.declined, isTrue);
      expect(record.answers, isEmpty);
    });
  });

  group('chatMessagesFromHistory — feedback', () {
    test('maps rating and comment onto ChatMessage.feedback', () {
      final messages = chatMessagesFromHistory([
        _msg(
          id: 'a1',
          role: 'assistant',
          content: 'Answer',
          feedback: (rating: 'negative', comment: 'wrong file'),
        ),
        _msg(
          id: 'a2',
          role: 'assistant',
          content: 'Answer',
          feedback: (rating: 'positive', comment: null),
        ),
      ]);

      expect(messages[0].feedback?.rating, MessageRating.negative);
      expect(messages[0].feedback?.comment, 'wrong file');
      expect(messages[1].feedback?.rating, MessageRating.positive);
      expect(messages[1].feedback?.comment, isNull);
    });

    test('absent feedback maps to null', () {
      final messages = chatMessagesFromHistory([
        _msg(id: 'a1', role: 'assistant', content: 'Answer'),
      ]);

      expect(messages.single.feedback, isNull);
    });

    test('an unknown rating maps to null instead of failing', () {
      final messages = chatMessagesFromHistory([
        _msg(
          id: 'a1',
          role: 'assistant',
          content: 'Answer',
          feedback: (rating: 'sideways', comment: null),
        ),
      ]);

      expect(messages.single.feedback, isNull);
    });
  });
}
