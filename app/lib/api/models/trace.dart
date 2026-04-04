/// A lightweight summary of an orchestration trace returned by `GET /api/traces`.
class TraceSummary {
  const TraceSummary({
    required this.traceId,
    required this.personaId,
    required this.startTime,
    required this.durationMs,
    this.skillName,
    required this.status,
    this.conversationId,
  });

  final String traceId;
  final String personaId;
  final DateTime startTime;

  /// Total turn duration in milliseconds.
  final int durationMs;

  /// The skill used during this trace, if any.
  final String? skillName;

  /// `"ok"` or `"error"`.
  final String status;

  final String? conversationId;

  bool get isError => status == 'error';

  factory TraceSummary.fromJson(Map<String, dynamic> json) {
    return TraceSummary(
      traceId: json['trace_id'] as String,
      personaId: json['persona_id'] as String,
      startTime: DateTime.parse(json['start_time'] as String),
      durationMs: (json['duration_ms'] as num).toInt(),
      skillName: json['skill_name'] as String?,
      status: json['status'] as String? ?? 'ok',
      conversationId: json['conversation_id'] as String?,
    );
  }
}

/// A trace with its full span breakdown, returned by `GET /api/traces/{id}`.
class TraceDetail {
  const TraceDetail({
    required this.traceId,
    required this.personaId,
    required this.startTime,
    required this.durationMs,
    required this.spans,
  });

  final String traceId;
  final String personaId;
  final DateTime startTime;
  final int durationMs;
  final List<SpanEntry> spans;

  factory TraceDetail.fromJson(Map<String, dynamic> json) {
    final spanList = (json['spans'] as List<dynamic>? ?? [])
        .cast<Map<String, dynamic>>()
        .map(SpanEntry.fromJson)
        .toList();

    return TraceDetail(
      traceId: json['trace_id'] as String,
      personaId: json['persona_id'] as String,
      startTime: DateTime.parse(json['start_time'] as String),
      durationMs: (json['duration_ms'] as num).toInt(),
      spans: spanList,
    );
  }
}

/// A single span within a trace.
class SpanEntry {
  const SpanEntry({
    required this.spanId,
    required this.name,
    required this.startTime,
    required this.durationMs,
    required this.attributes,
  });

  final String spanId;

  /// Span name, e.g. `"llm_turn"` or `"tool:file-read"`.
  final String name;

  final DateTime startTime;
  final int durationMs;

  /// Arbitrary key-value attributes associated with the span.
  final Map<String, String> attributes;

  factory SpanEntry.fromJson(Map<String, dynamic> json) {
    final rawAttrs = json['attributes'] as Map<String, dynamic>? ?? {};
    final attrs = rawAttrs.map(
      (key, value) => MapEntry(key, value?.toString() ?? ''),
    );

    return SpanEntry(
      spanId: json['span_id'] as String,
      name: json['name'] as String,
      startTime: DateTime.parse(json['start_time'] as String),
      durationMs: (json['duration_ms'] as num).toInt(),
      attributes: attrs,
    );
  }
}
