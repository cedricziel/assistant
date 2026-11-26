// Tests for the ApiClient wrappers around the interaction and tool-permission
// rule routes: request shape on the wire and mapping of the responses.

import 'dart:convert';
import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart'
    show InteractionStatusView, RuleScope;
import 'package:assistant_ui/models.dart';
import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/interaction_answer.dart';

/// Records every request and answers with [status] and the JSON [body].
class _RecordingAdapter implements HttpClientAdapter {
  _RecordingAdapter(this.body, {this.status = 200});

  final Object? body;
  final int status;
  final List<RequestOptions> requests = [];
  final List<String> bodies = [];

  RequestOptions get request => requests.single;
  Object? get sentJson => jsonDecode(bodies.single);

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<dynamic>? cancelFuture,
  ) async {
    requests.add(options);
    final sent = <int>[];
    if (requestStream != null) {
      await for (final chunk in requestStream) {
        sent.addAll(chunk);
      }
    }
    bodies.add(utf8.decode(sent));
    return ResponseBody.fromString(
      body == null ? '' : jsonEncode(body),
      status,
      headers: {
        'content-type': ['application/json'],
      },
    );
  }

  @override
  void close({bool force = false}) {}
}

const _detail = {
  'interaction_id': 'i-1',
  'conversation_id': 'c-1',
  'run_id': 'run-1',
  'kind': 'tool_approval',
  'status': 'resolved',
  'renders_card': true,
  'created_at': '2026-01-02T03:04:05Z',
  'expires_at': '2026-01-02T03:09:05Z',
  'resolved_at': '2026-01-02T03:05:00Z',
  'request': {'tool_name': 'bash'},
  'response': {'decision': 'allow_once'},
};

(ApiClient, _RecordingAdapter) _client(Object? body, {int status = 200}) {
  final adapter = _RecordingAdapter(body, status: status);
  final client = ApiClient(baseUrl: 'http://fake', token: 'tok');
  client.dioForTesting.httpClientAdapter = adapter;
  return (client, adapter);
}

void main() {
  group('respondToInteraction', () {
    const path = '/api/conversations/c-1/turns/run-1/interactions/i-1/respond';

    test('posts a tool approval decision and returns the detail', () async {
      final (client, adapter) = _client(_detail);

      final detail = await client.respondToInteraction(
        'c-1',
        'run-1',
        'i-1',
        const ToolApprovalAnswer(ApprovalDecision.allowAlways),
      );

      expect(adapter.request.method, 'POST');
      expect(adapter.request.path, path);
      expect(adapter.sentJson, {
        'kind': 'tool_approval',
        'decision': 'allow_always',
      });
      expect(detail.interactionId, 'i-1');
      expect(detail.status, InteractionStatusView.resolved);
    });

    test('posts question answers', () async {
      final (client, adapter) = _client(_detail);

      await client.respondToInteraction(
        'c-1',
        'run-1',
        'i-1',
        const QuestionAnswers([
          QuestionAnswer(questionId: 'q1', selectedOptionIds: ['o1']),
          QuestionAnswer(questionId: 'q2', text: 'main'),
        ]),
      );

      expect(adapter.sentJson, {
        'kind': 'question',
        'answers': [
          {
            'question_id': 'q1',
            'selected_option_ids': ['o1'],
          },
          {'question_id': 'q2', 'selected_option_ids': [], 'text': 'main'},
        ],
      });
    });

    test('posts a decline', () async {
      final (client, adapter) = _client(_detail);

      await client.respondToInteraction(
        'c-1',
        'run-1',
        'i-1',
        const DeclineAnswer(),
      );

      expect(adapter.sentJson, {'kind': 'decline'});
    });

    test('lets the HTTP error through so the card can show it', () async {
      final (client, _) = _client({'error': 'not found'}, status: 404);

      await expectLater(
        client.respondToInteraction(
          'c-1',
          'run-1',
          'i-1',
          const DeclineAnswer(),
        ),
        throwsA(
          isA<DioException>().having(
            (e) => e.response?.statusCode,
            'status',
            404,
          ),
        ),
      );
    });
  });

  group('listConversationInteractions', () {
    test('lists every interaction without a filter', () async {
      final (client, adapter) = _client([_detail]);

      final list = await client.listConversationInteractions('c-1');

      expect(adapter.request.method, 'GET');
      expect(adapter.request.path, '/api/conversations/c-1/interactions');
      expect(adapter.request.queryParameters, isEmpty);
      expect(list.single.interactionId, 'i-1');
    });

    test('passes the status filter', () async {
      final (client, adapter) = _client(<Object>[]);

      final list = await client.listConversationInteractions(
        'c-1',
        status: InteractionStatusView.pending,
      );

      expect(adapter.request.queryParameters, {'status': 'pending'});
      expect(list, isEmpty);
    });
  });

  group('tool permission rules', () {
    const rule = {
      'id': 'r-1',
      'user_id': 'u-1',
      'tool_name': 'bash',
      'rule': 'bash(git status*)',
      'effect': 'allow',
      'created_at': '2026-01-02T03:04:05Z',
    };

    test('listToolPermissionRules defaults to the caller\'s rules', () async {
      final (client, adapter) = _client([rule]);

      final rules = await client.listToolPermissionRules();

      expect(adapter.request.method, 'GET');
      expect(adapter.request.path, '/api/tool-permission-rules');
      expect(adapter.request.queryParameters, isEmpty);
      expect(rules.single.rule, 'bash(git status*)');
    });

    test('listToolPermissionRules passes the scope', () async {
      final (client, adapter) = _client([rule]);

      await client.listToolPermissionRules(scope: RuleScope.space);

      expect(adapter.request.queryParameters, {'scope': 'space'});
    });

    test('deleteToolPermissionRule deletes by id', () async {
      final (client, adapter) = _client(null, status: 204);

      await client.deleteToolPermissionRule('r-1');

      expect(adapter.request.method, 'DELETE');
      expect(adapter.request.path, '/api/tool-permission-rules/r-1');
    });
  });
}
