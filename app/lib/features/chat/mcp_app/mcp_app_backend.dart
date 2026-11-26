import 'package:assistant_api/assistant_api.dart';
import 'package:built_value/json_object.dart';
import 'package:dio/dio.dart';

import 'mcp_app_bridge.dart';

/// Serves an app's calls through the tool call's `app/` routes.
class ApiMcpAppBackend implements McpAppBackend {
  const ApiMcpAppBackend(
    this._api, {
    required this.conversationId,
    required this.toolCallId,
  });

  final ConversationsApi _api;
  final String conversationId;
  final String toolCallId;

  @override
  Future<Map<String, Object?>> callTool(
    String name,
    Map<String, Object?> arguments, {
    required bool confirmed,
  }) => _send(
    () => _api.callMcpAppTool(
      conversationId: conversationId,
      toolCallId: toolCallId,
      callMcpAppToolRequest: CallMcpAppToolRequest(
        (b) => b
          ..name = name
          ..arguments = JsonObject(arguments)
          ..confirmed = confirmed,
      ),
    ),
  );

  @override
  Future<Map<String, Object?>> readResource(String uri) => _send(
    () => _api.readMcpAppResource(
      conversationId: conversationId,
      toolCallId: toolCallId,
      readMcpAppResourceRequest: ReadMcpAppResourceRequest((b) => b..uri = uri),
    ),
  );

  Future<Map<String, Object?>> _send(
    Future<Response<JsonObject>> Function() request,
  ) async {
    try {
      final response = await request();
      final data = response.data?.value;
      return data is Map ? data.cast<String, Object?>() : const {};
    } on DioException catch (e) {
      final body = e.response?.data;
      final reason = body is Map ? body['error'] : null;
      throw McpAppRefused(
        e.response?.statusCode,
        reason is String ? reason : e.message ?? 'The request failed',
      );
    }
  }
}
