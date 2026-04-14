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
/// `{"role":"assistant","content":"<full text>"}`.  On receiving this event
/// the caller should discard the accumulated buffer and persist `content`.
class DoneEvent extends StreamEvent {
  const DoneEvent({required this.role, required this.content});

  final String role;

  /// The complete, server-authoritative reply text.
  final String content;

  factory DoneEvent.fromJson(Map<String, dynamic> json) {
    return DoneEvent(
      role: json['role'] as String? ?? 'assistant',
      content: json['content'] as String? ?? '',
    );
  }
}

/// An error that occurred while processing the stream.
///
/// May be produced by the client-side SSE parser when the stream closes
/// unexpectedly or when an HTTP error status is received.
class ErrorEvent extends StreamEvent {
  const ErrorEvent(this.message);

  final String message;
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
