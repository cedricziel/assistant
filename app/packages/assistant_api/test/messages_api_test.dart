import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for MessagesApi
void main() {
  final instance = AssistantApi().getMessagesApi();

  group(MessagesApi, () {
    // `POST /message/send` -- Sends a message to the agent (unary).
    //
    // Creates a task, records the user message, transitions to Working, produces an agent reply, and returns the final task state.
    //
    //Future<SendMessageResponse> sendMessage(SendMessageRequest sendMessageRequest) async
    test('test sendMessage', () async {
      // TODO
    });

    // `POST /message/stream` -- Sends a message with streaming response (SSE).
    //
    //Future sendMessageStreaming(SendMessageRequest sendMessageRequest) async
    test('test sendMessageStreaming', () async {
      // TODO
    });
  });
}
