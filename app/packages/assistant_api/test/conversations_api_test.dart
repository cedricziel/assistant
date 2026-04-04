import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';


/// tests for ConversationsApi
void main() {
  final instance = AssistantApi().getConversationsApi();

  group(ConversationsApi, () {
    // `POST /api/conversations` — create a new conversation.
    //
    //Future<ConversationSummary> createConversation(CreateConversationRequest createConversationRequest) async
    test('test createConversation', () async {
      // TODO
    });

    // `DELETE /api/conversations/{id}` — delete a conversation and all its messages.
    //
    //Future deleteConversation(String id) async
    test('test deleteConversation', () async {
      // TODO
    });

    // `GET /api/conversations/{id}` — get a conversation and its message history.
    //
    //Future<ConversationDetail> getConversation(String id) async
    test('test getConversation', () async {
      // TODO
    });

    // `GET /api/conversations` — list all conversations, newest first.
    //
    //Future<BuiltList<ConversationSummary>> listConversations() async
    test('test listConversations', () async {
      // TODO
    });

    // `POST /api/conversations/{id}/messages` — send a message and stream the response.
    //
    // The response is a `text/event-stream` (SSE) with two event types: - `event: token` — incremental assistant token (data is plain text) - `event: done`  — final JSON object: `{\"role\":\"assistant\",\"content\":\"...\"}`
    //
    //Future sendMessage(String id, ApiSendMessageRequest apiSendMessageRequest) async
    test('test sendMessage', () async {
      // TODO
    });

    // `PATCH /api/conversations/{id}` — update a conversation's title.
    //
    //Future<ConversationSummary> updateConversation(String id, UpdateConversationRequest updateConversationRequest) async
    test('test updateConversation', () async {
      // TODO
    });

  });
}
