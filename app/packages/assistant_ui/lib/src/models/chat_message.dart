import 'dart:typed_data';

import 'mcp_app.dart';
import 'message_timing.dart';

// -- Chat messages -----------------------------------------------------------

/// Id of the in-flight assistant message while a reply is streaming.
const streamingPlaceholderId = 'assistant-streaming';

/// Delivery status of a user message.
enum MessageStatus {
  /// The message is currently in-flight (streaming response in progress).
  sending,

  /// The message was acknowledged by the server (DoneEvent received).
  ok,

  /// The message failed to deliver (network or server error).
  failed,

  /// The assistant reply was cut short by a Stop; the content is the partial.
  stopped,
}

/// Status of a single tool call within an assistant message.
enum ToolCallStatus { pending, ok, error, denied, cancelled }

/// A single tool invocation recorded on an assistant message.
class ToolCallRecord {
  ToolCallRecord({
    required this.toolName,
    required this.status,
    this.toolCallId,
    this.arguments,
    this.result,
    this.data,
    this.app,
    DateTime? startedAt,
    this.duration,
  }) : startedAt = startedAt ?? DateTime.now();

  final String toolName;

  /// Provider-assigned tool-call ID (e.g. Anthropic `tool_use_id`).
  /// Used to correlate StatusEvent → ToolResultEvent when available.
  final String? toolCallId;

  /// Mutable so the pending chip can be updated to a resolved status in place
  /// during streaming, matching the pattern used for [ChatMessage.content].
  ToolCallStatus status;

  /// The JSON arguments passed to the tool (may be null while pending).
  Map<String, dynamic>? arguments;

  /// The tool's output, truncated for display (populated on completion).
  String? result;

  /// Structured output of the tool, a JSON object with a `kind` tag.
  ///
  /// Null while pending, for text-only tools, when the server dropped
  /// oversized data, and for calls that predate structured data.
  Map<String, Object?>? data;

  /// The MCP App the tool declared for its result, rendered below the chip.
  McpApp? app;

  /// When this tool call started (set on StatusEvent).
  final DateTime startedAt;

  /// Elapsed time from start to completion. Null while pending.
  Duration? duration;
}

/// The kind of entry shown in the chat timeline.
enum TimelineEntryType {
  message,
  thinking,
  toolCall,
  subagent,
  command,

  /// An interaction request that has no tool call to attach to. Only the
  /// fallback: a request with a `tool_call_id` renders in that tool call's
  /// footer.
  interaction,
}

/// Metadata for an image attachment on a message.
class ChatAttachment {
  const ChatAttachment({
    required this.id,
    required this.filename,
    required this.mimeType,
    required this.url,
  });

  final String id;
  final String filename;
  final String mimeType;
  final String url;
}

/// A message shown in the chat UI (may be a streaming partial).
class ChatMessage {
  ChatMessage({
    required this.id,
    required this.role,
    required this.content,
    this.isStreaming = false,
    this.status = MessageStatus.ok,
    this.audioId,
    this.ttsAvailable = false,
    this.tokenStream,
    this.audioBytes,
    this.audioMimeType,
    this.timelineType = TimelineEntryType.message,
    this.thinkingContent,
    this.subagentId,
    this.subagentTask,
    this.subagentSummary,
    this.commandName,
    this.commandAckText,
    this.timing,
    List<ToolCallRecord>? toolCalls,
    List<ChatAttachment>? attachments,
  }) : toolCalls = toolCalls ?? [],
       attachments = attachments ?? [];

  final String id;
  final String role;
  String content;
  bool isStreaming;

  /// Whether this timeline entry has been superseded by newer activity
  /// (e.g. final answer streaming has begun). Used to derive [EntryState.stale].
  bool isStale = false;

  MessageStatus status;

  /// Non-null when the server has produced audio for this assistant message.
  final String? audioId;

  /// Whether TTS audio can be synthesised for this message.
  /// Set from history (`MessageSummary.ttsAvailable`) and updated during
  /// streaming when an [AudioReadyEvent] is received or the stream completes
  /// with content.
  bool ttsAvailable;

  /// Tool calls associated with this assistant message, in invocation order.
  /// Populated during streaming from [StatusEvent] and [ToolResultEvent].
  List<ToolCallRecord> toolCalls;

  /// Live stream of token chunks during streaming. Used by [StreamMarkdown]
  /// for throttled incremental rendering. Null for non-streaming messages.
  Stream<String>? tokenStream;

  /// Image attachments linked to this message.
  List<ChatAttachment> attachments;

  /// Raw audio bytes for user voice messages (in-memory only, not persisted).
  final Uint8List? audioBytes;

  /// MIME type of [audioBytes] (e.g. `audio/webm`, `audio/mp4`).
  final String? audioMimeType;

  /// The kind of timeline entry this message represents.
  final TimelineEntryType timelineType;

  /// Reasoning text for [TimelineEntryType.thinking] entries.
  final String? thinkingContent;

  /// Live stream of thinking tokens during streaming. Used by the timeline
  /// entry for incremental rendering. Null for non-streaming messages.
  Stream<String>? thinkingTokenStream;

  /// Subagent identifier for [TimelineEntryType.subagent] entries.
  final String? subagentId;

  /// Subagent task description for [TimelineEntryType.subagent] entries.
  final String? subagentTask;

  /// Subagent completion summary for [TimelineEntryType.subagent] entries.
  String? subagentSummary;

  /// Accumulated content tokens produced by a subagent.
  String subagentContent = '';

  /// Accumulated thinking text produced by a subagent.
  String subagentThinking = '';

  /// Tool calls executed by a subagent, in order.
  List<ToolCallRecord> subagentToolCalls = [];

  /// Command name for [TimelineEntryType.command] entries (without `/`).
  final String? commandName;

  /// Acknowledgement text for [TimelineEntryType.command] entries.
  final String? commandAckText;

  /// Server-measured response timing, set from the `done` frame or history.
  ///
  /// Mutable like [content], so a finished stream can attach it in place.
  MessageTiming? timing;

  bool get isUser => role == 'user';
  bool get isAssistant => role == 'assistant';

  ChatMessage copyWith({
    String? content,
    bool? isStreaming,
    MessageStatus? status,
    String? audioId,
    bool? ttsAvailable,
    List<ToolCallRecord>? toolCalls,
    Stream<String>? tokenStream,
    bool clearTokenStream = false,
    List<ChatAttachment>? attachments,
    Uint8List? audioBytes,
    String? audioMimeType,
    TimelineEntryType? timelineType,
    String? thinkingContent,
    String? subagentId,
    String? subagentTask,
    String? subagentSummary,
    MessageTiming? timing,
  }) {
    return ChatMessage(
      id: id,
      role: role,
      content: content ?? this.content,
      isStreaming: isStreaming ?? this.isStreaming,
      status: status ?? this.status,
      audioId: audioId ?? this.audioId,
      ttsAvailable: ttsAvailable ?? this.ttsAvailable,
      toolCalls: toolCalls ?? this.toolCalls,
      tokenStream: clearTokenStream ? null : (tokenStream ?? this.tokenStream),
      attachments: attachments ?? this.attachments,
      audioBytes: audioBytes ?? this.audioBytes,
      audioMimeType: audioMimeType ?? this.audioMimeType,
      timelineType: timelineType ?? this.timelineType,
      thinkingContent: thinkingContent ?? this.thinkingContent,
      subagentId: subagentId ?? this.subagentId,
      subagentTask: subagentTask ?? this.subagentTask,
      subagentSummary: subagentSummary ?? this.subagentSummary,
      timing: timing ?? this.timing,
    );
  }
}

/// A message waiting in the send queue.
///
/// Stores the message text together with the [conversationId] that was active
/// at enqueue time, so messages are always routed to the correct conversation
/// even if the user navigates away while the queue is draining.
class PendingMessage {
  const PendingMessage({
    required this.text,
    required this.conversationId,
    this.attachmentIds = const [],
    this.attachments = const [],
  });
  final String text;
  final String conversationId;
  final List<String> attachmentIds;

  /// Pre-built attachment metadata from upload responses, used to show
  /// thumbnails immediately in the user bubble without waiting for a reload.
  final List<ChatAttachment> attachments;
}
