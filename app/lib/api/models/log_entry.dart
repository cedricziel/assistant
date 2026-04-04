/// A structured log line returned by `GET /api/logs`.
class LogEntry {
  const LogEntry({
    required this.id,
    required this.timestamp,
    required this.severity,
    required this.target,
    required this.message,
    required this.fields,
    this.traceId,
    this.conversationId,
  });

  final String id;
  final DateTime timestamp;

  /// Severity level: `"TRACE"`, `"DEBUG"`, `"INFO"`, `"WARN"`, or `"ERROR"`.
  final String severity;

  /// Rust module path that emitted the log, e.g.
  /// `"assistant_runtime::orchestrator"`.
  final String target;

  final String message;

  /// Structured key-value fields attached to the log line.
  final Map<String, String> fields;

  final String? traceId;
  final String? conversationId;

  factory LogEntry.fromJson(Map<String, dynamic> json) {
    final rawFields = json['fields'] as Map<String, dynamic>? ?? {};
    final fields = rawFields.map(
      (key, value) => MapEntry(key, value?.toString() ?? ''),
    );

    return LogEntry(
      id: json['id'] as String,
      timestamp: DateTime.parse(json['timestamp'] as String),
      severity: json['severity'] as String? ?? 'INFO',
      target: json['target'] as String? ?? '',
      message: json['message'] as String? ?? '',
      fields: fields,
      traceId: json['trace_id'] as String?,
      conversationId: json['conversation_id'] as String?,
    );
  }
}
