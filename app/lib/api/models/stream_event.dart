/// Sealed class hierarchy representing a single SSE event from the chat API.
///
/// Events arrive from `POST /api/conversations/{id}/messages` as a
/// `text/event-stream` (Server-Sent Events) response.
sealed class StreamEvent {
  const StreamEvent();
}

/// An incremental text token emitted by the assistant.
///
/// Corresponds to `event:token` in the SSE stream.  The caller should
/// accumulate these into a display buffer to show the streaming response.
class TokenEvent extends StreamEvent {
  const TokenEvent(this.token);

  /// The incremental text chunk (may be a single word, part of a word, or
  /// whitespace).
  final String token;
}

/// A status update emitted while the assistant is processing.
///
/// Corresponds to `event:status` in the SSE stream.  The caller may display
/// this as a transient status indicator (e.g. "Calling tool: web-search").
class StatusEvent extends StreamEvent {
  const StatusEvent(this.message);

  /// Human-readable status message.
  final String message;
}

/// A tool execution completed.
///
/// Corresponds to `event:tool_result` in the SSE stream.  The JSON body is
/// `{"tool_name":"...","status":"ok"|"error"|"denied"}`.
class ToolResultEvent extends StreamEvent {
  const ToolResultEvent({required this.toolName, required this.status});

  /// The name of the tool that was called.
  final String toolName;

  /// `"ok"` on success, `"error"` or `"denied"` otherwise.
  final String status;

  factory ToolResultEvent.fromJson(Map<String, dynamic> json) {
    return ToolResultEvent(
      toolName: json['tool_name'] as String? ?? '',
      status: json['status'] as String? ?? 'ok',
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
  const DoneEvent({required this.role, required this.content, this.messageId});

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
  const RunStartedEvent(this.runId);

  /// The UUID of the orchestrator run.
  final String runId;
}

/// An error that occurred while processing the stream.
///
/// May be produced by the client-side SSE parser when the stream closes
/// unexpectedly or when an HTTP error status is received.
class ErrorEvent extends StreamEvent {
  const ErrorEvent(this.message);

  final String message;
}

/// The voice endpoint has transcribed the user's audio.
///
/// Corresponds to `event:transcript` in the SSE stream emitted by
/// `POST /api/conversations/{id}/voice`.  The JSON body is
/// `{"role":"user","content":"<transcribed text>"}`.
class TranscriptEvent extends StreamEvent {
  const TranscriptEvent(this.transcript);

  /// The transcribed text of the user's spoken message.
  final String transcript;

  factory TranscriptEvent.fromJson(Map<String, dynamic> json) {
    return TranscriptEvent(json['content'] as String? ?? '');
  }
}

/// The server has produced audio for the most recent assistant response.
///
/// Corresponds to `event:audio_ready` in the SSE stream.  The JSON body is
/// `{"audio_id":"<uuid>"}`.
class AudioReadyEvent extends StreamEvent {
  const AudioReadyEvent(this.audioId);

  /// Opaque identifier used with GET /api/audio/{audioId}.
  final String audioId;

  factory AudioReadyEvent.fromJson(Map<String, dynamic> json) {
    return AudioReadyEvent(json['audio_id'] as String? ?? '');
  }
}
