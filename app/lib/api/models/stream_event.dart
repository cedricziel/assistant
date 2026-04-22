/// Sealed class hierarchy representing a single SSE event from the chat API.
///
/// Events arrive from `POST /api/conversations/{id}/messages` as a
/// `text/event-stream` (Server-Sent Events) response.
sealed class StreamEvent {
  const StreamEvent({this.sequenceId});

  /// The server-assigned event sequence number parsed from the SSE `id:` field.
  ///
  /// `null` when the server did not include an `id:` line (e.g. ephemeral
  /// thinking tokens or older servers).
  final int? sequenceId;
}

// -- Conversation list stream events -----------------------------------------

/// Sealed class for events from `GET /api/conversations/stream`.
sealed class ConversationListEvent {
  const ConversationListEvent();
}

/// Full snapshot of the conversation list, sent on initial connect.
class ConversationSnapshotEvent extends ConversationListEvent {
  const ConversationSnapshotEvent(this.conversations);

  final List<ConversationListEntry> conversations;

  factory ConversationSnapshotEvent.fromJson(List<dynamic> json) {
    return ConversationSnapshotEvent(
      json
          .cast<Map<String, dynamic>>()
          .map(ConversationListEntry.fromJson)
          .toList(),
    );
  }
}

/// A conversation was created or updated.
class ConversationUpsertedEvent extends ConversationListEvent {
  const ConversationUpsertedEvent(this.conversation);

  final ConversationListEntry conversation;

  factory ConversationUpsertedEvent.fromJson(Map<String, dynamic> json) {
    return ConversationUpsertedEvent(ConversationListEntry.fromJson(json));
  }
}

/// A conversation was deleted.
class ConversationDeletedEvent extends ConversationListEvent {
  const ConversationDeletedEvent(this.conversationId);

  final String conversationId;

  factory ConversationDeletedEvent.fromJson(Map<String, dynamic> json) {
    final conversationId = json['conversation_id'] as String?;
    if (conversationId == null || conversationId.isEmpty) {
      throw const FormatException('conversation_id is required');
    }
    return ConversationDeletedEvent(conversationId);
  }
}

/// Lightweight conversation summary carried in stream events.
class ConversationListEntry {
  const ConversationListEntry({
    required this.id,
    required this.title,
    required this.createdAt,
    required this.updatedAt,
  });

  final String id;
  final String title;
  final DateTime createdAt;
  final DateTime updatedAt;

  factory ConversationListEntry.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    if (id == null || id.isEmpty) {
      throw const FormatException('id is required');
    }
    return ConversationListEntry(
      id: id,
      title: json['title'] as String? ?? 'Untitled',
      createdAt: DateTime.parse(json['created_at'] as String),
      updatedAt: DateTime.parse(json['updated_at'] as String),
    );
  }
}

/// An incremental text token emitted by the assistant.
///
/// Corresponds to `event:token` in the SSE stream.  The caller should
/// accumulate these into a display buffer to show the streaming response.
class TokenEvent extends StreamEvent {
  const TokenEvent(this.token, {super.sequenceId});

  /// The incremental text chunk (may be a single word, part of a word, or
  /// whitespace).
  final String token;
}

/// A status update emitted while the assistant is processing.
///
/// Corresponds to `event:status` in the SSE stream.  The caller may display
/// this as a transient status indicator (e.g. "Calling tool: web-search").
///
/// The data payload is JSON: `{"message":"...","tool_call_id":"..."}`.
/// Older servers may send a plain string — the parser falls back gracefully.
class StatusEvent extends StreamEvent {
  const StatusEvent(this.message, {this.toolCallId, super.sequenceId});

  /// Human-readable status message.
  final String message;

  /// Provider-assigned tool-call ID (e.g. Anthropic `tool_use_id`).
  /// `null` when the server does not include it.
  final String? toolCallId;

  factory StatusEvent.fromJson(Map<String, dynamic> json) {
    return StatusEvent(
      json['message'] as String? ?? '',
      toolCallId: json['tool_call_id'] as String?,
    );
  }
}

/// A tool execution completed.
///
/// Corresponds to `event:tool_result` in the SSE stream.  The JSON body is
/// `{"tool_name":"...","status":"ok"|"error"|"denied","arguments":{...},"result":"...","tool_call_id":"..."}`.
class ToolResultEvent extends StreamEvent {
  const ToolResultEvent({
    required this.toolName,
    required this.status,
    this.arguments,
    this.result,
    this.toolCallId,
    super.sequenceId,
  });

  /// The name of the tool that was called.
  final String toolName;

  /// `"ok"` on success, `"error"` or `"denied"` otherwise.
  final String status;

  /// The JSON arguments passed to the tool (may be null).
  final Map<String, dynamic>? arguments;

  /// The tool's output, truncated for display (may be null).
  final String? result;

  /// Provider-assigned tool-call ID (e.g. Anthropic `tool_use_id`).
  /// `null` for providers that do not use IDs (Ollama) or older servers.
  final String? toolCallId;

  factory ToolResultEvent.fromJson(Map<String, dynamic> json) {
    return ToolResultEvent(
      toolName: json['tool_name'] as String? ?? '',
      status: json['status'] as String? ?? 'ok',
      arguments: json['arguments'] as Map<String, dynamic>?,
      result: json['result'] as String?,
      toolCallId: json['tool_call_id'] as String?,
    );
  }
}

/// The stream is complete; contains the full, canonical reply.
///
/// Corresponds to `event:done` in the SSE stream.  The JSON body is
/// `{"role":"assistant","content":"<full text>","message_id":"<uuid>"}`.
/// On receiving this event the caller should discard the accumulated buffer
/// and persist `content`.  `messageId` is the DB UUID of the saved assistant
/// message and should be used as [ChatMessage.id] when present.
class DoneEvent extends StreamEvent {
  const DoneEvent({
    required this.role,
    required this.content,
    this.messageId,
    super.sequenceId,
  });

  final String role;

  /// The complete, server-authoritative reply text.
  final String content;

  /// The UUID of the persisted assistant message in the database.
  /// Null when the server did not include a `message_id` field (old server).
  final String? messageId;

  factory DoneEvent.fromJson(Map<String, dynamic> json) {
    return DoneEvent(
      role: json['role'] as String? ?? 'assistant',
      content: json['content'] as String? ?? '',
      messageId: json['message_id'] as String?,
    );
  }
}

/// The server has started a new orchestrator run.
///
/// Corresponds to `event:run_started` in the SSE stream.  The JSON body is
/// `{"run_id":"<uuid>"}`.  Clients should store this ID so they can reconnect
/// via the event-log replay endpoint if the connection drops.
class RunStartedEvent extends StreamEvent {
  const RunStartedEvent(this.runId, {super.sequenceId});

  /// The UUID of the orchestrator run.
  final String runId;
}

/// An error that occurred while processing the stream.
///
/// May be produced by the client-side SSE parser when the stream closes
/// unexpectedly or when an HTTP error status is received.
class ErrorEvent extends StreamEvent {
  const ErrorEvent(this.message, {super.sequenceId});

  final String message;
}

/// The voice endpoint has transcribed the user's audio.
///
/// Corresponds to `event:transcript` in the SSE stream emitted by
/// `POST /api/conversations/{id}/voice`.  The JSON body is
/// `{"role":"user","content":"<transcribed text>"}`.
class TranscriptEvent extends StreamEvent {
  const TranscriptEvent(this.transcript, {super.sequenceId});

  /// The transcribed text of the user's spoken message.
  final String transcript;

  factory TranscriptEvent.fromJson(Map<String, dynamic> json) {
    return TranscriptEvent(json['content'] as String? ?? '');
  }
}

/// Internal reasoning / chain-of-thought produced by the LLM.
///
/// Corresponds to `event:thinking` in the SSE stream.  The JSON body is
/// `{"content":"<thinking text>"}`.
class ThinkingEvent extends StreamEvent {
  const ThinkingEvent(this.content, {super.sequenceId});

  /// The reasoning text produced by the LLM.
  final String content;

  factory ThinkingEvent.fromJson(Map<String, dynamic> json) {
    return ThinkingEvent(json['content'] as String? ?? '');
  }
}

/// A subagent process was started.
///
/// Corresponds to `event:subagent_started` in the SSE stream.  The JSON body
/// is `{"agent_id":"...","task":"..."}`.
class SubagentStartedEvent extends StreamEvent {
  const SubagentStartedEvent({
    required this.agentId,
    required this.task,
    super.sequenceId,
  });

  final String agentId;
  final String task;

  factory SubagentStartedEvent.fromJson(Map<String, dynamic> json) {
    return SubagentStartedEvent(
      agentId: json['agent_id'] as String? ?? '',
      task: json['task'] as String? ?? '',
    );
  }
}

/// A subagent process completed.
///
/// Corresponds to `event:subagent_completed` in the SSE stream.  The JSON
/// body is `{"agent_id":"...","status":"ok"|"error","summary":"..."}`.
class SubagentCompletedEvent extends StreamEvent {
  const SubagentCompletedEvent({
    required this.agentId,
    required this.status,
    required this.summary,
    super.sequenceId,
  });

  final String agentId;
  final String status;
  final String summary;

  factory SubagentCompletedEvent.fromJson(Map<String, dynamic> json) {
    return SubagentCompletedEvent(
      agentId: json['agent_id'] as String? ?? '',
      status: json['status'] as String? ?? 'ok',
      summary: json['summary'] as String? ?? '',
    );
  }
}

/// A skill run completed (success or failure).
///
/// Corresponds to `event:skill_complete` in the SSE stream.  The JSON body
/// is `{"skill_name":"...","success":true|false,"summary":"..."}`.
class SkillCompleteEvent extends StreamEvent {
  const SkillCompleteEvent({
    required this.skillName,
    required this.success,
    required this.summary,
    super.sequenceId,
  });

  final String skillName;
  final bool success;
  final String summary;

  factory SkillCompleteEvent.fromJson(Map<String, dynamic> json) {
    return SkillCompleteEvent(
      skillName: json['skill_name'] as String? ?? '',
      success: json['success'] as bool? ?? false,
      summary: json['summary'] as String? ?? '',
    );
  }
}

/// The orchestrator encountered a critical error.
///
/// Corresponds to `event:agent_error` in the SSE stream.  The data is a
/// plain-text error message.
class AgentErrorEvent extends StreamEvent {
  const AgentErrorEvent(this.message, {super.sequenceId});

  final String message;
}

/// An incremental text token produced by a running subagent.
///
/// Corresponds to `event:subagent_token` in the SSE stream.  The JSON body is
/// `{"agent_id":"...","data":{"content":"..."}}`.
class SubagentTokenEvent extends StreamEvent {
  const SubagentTokenEvent({
    required this.agentId,
    required this.content,
    super.sequenceId,
  });

  final String agentId;
  final String content;

  factory SubagentTokenEvent.fromJson(Map<String, dynamic> json) {
    final data = json['data'] as Map<String, dynamic>? ?? {};
    return SubagentTokenEvent(
      agentId: json['agent_id'] as String? ?? '',
      content: data['content'] as String? ?? '',
    );
  }
}

/// Internal reasoning produced by a running subagent.
///
/// Corresponds to `event:subagent_thinking` in the SSE stream.  The JSON body
/// is `{"agent_id":"...","data":{"content":"..."}}`.
class SubagentThinkingEvent extends StreamEvent {
  const SubagentThinkingEvent({
    required this.agentId,
    required this.content,
    super.sequenceId,
  });

  final String agentId;
  final String content;

  factory SubagentThinkingEvent.fromJson(Map<String, dynamic> json) {
    final data = json['data'] as Map<String, dynamic>? ?? {};
    return SubagentThinkingEvent(
      agentId: json['agent_id'] as String? ?? '',
      content: data['content'] as String? ?? '',
    );
  }
}

/// A tool execution completed within a running subagent.
///
/// Corresponds to `event:subagent_tool_result` in the SSE stream.  The JSON
/// body is `{"agent_id":"...","data":{"tool_name":"...","status":"...","arguments":{...},"result":"..."}}`.
class SubagentToolResultEvent extends StreamEvent {
  const SubagentToolResultEvent({
    required this.agentId,
    required this.toolName,
    required this.status,
    this.arguments,
    this.result,
    super.sequenceId,
  });

  final String agentId;
  final String toolName;
  final String status;
  final Map<String, dynamic>? arguments;
  final String? result;

  factory SubagentToolResultEvent.fromJson(Map<String, dynamic> json) {
    final data = json['data'] as Map<String, dynamic>? ?? {};
    return SubagentToolResultEvent(
      agentId: json['agent_id'] as String? ?? '',
      toolName: data['tool_name'] as String? ?? '',
      status: data['status'] as String? ?? 'ok',
      arguments: data['arguments'] as Map<String, dynamic>?,
      result: data['result'] as String?,
    );
  }
}

/// A status update from a running subagent.
///
/// Corresponds to `event:subagent_status` in the SSE stream.  The JSON body
/// is `{"agent_id":"...","data":{"message":"..."}}`.
class SubagentStatusEvent extends StreamEvent {
  const SubagentStatusEvent({
    required this.agentId,
    required this.message,
    super.sequenceId,
  });

  final String agentId;
  final String message;

  factory SubagentStatusEvent.fromJson(Map<String, dynamic> json) {
    final data = json['data'] as Map<String, dynamic>? ?? {};
    return SubagentStatusEvent(
      agentId: json['agent_id'] as String? ?? '',
      message: data['message'] as String? ?? '',
    );
  }
}

/// The server has produced audio for the most recent assistant response.
///
/// Corresponds to `event:audio_ready` in the SSE stream.  The JSON body is
/// `{"audio_id":"<uuid>"}`.
class AudioReadyEvent extends StreamEvent {
  const AudioReadyEvent(this.audioId, {super.sequenceId});

  /// Opaque identifier used with GET /api/audio/{audioId}.
  final String audioId;

  factory AudioReadyEvent.fromJson(Map<String, dynamic> json) {
    return AudioReadyEvent(json['audio_id'] as String? ?? '');
  }
}
