import 'dart:convert';

import '../client.dart';
import '../models/log_entry.dart';
import 'conversations.dart';

/// Filter parameters for `GET /api/logs`.
class LogFilters {
  const LogFilters({
    this.limit,
    this.offset,
    this.search,
    this.severity,
    this.since,
    this.until,
    this.traceId,
    this.conversation,
  });

  final int? limit;
  final int? offset;

  /// Keyword filter applied to message and fields.
  final String? search;

  /// Minimum severity: `"TRACE"`, `"DEBUG"`, `"INFO"`, `"WARN"`, `"ERROR"`.
  final String? severity;
  final DateTime? since;
  final DateTime? until;
  final String? traceId;
  final String? conversation;

  Map<String, String?> toQueryParams() => {
        if (limit != null) 'limit': limit.toString(),
        if (offset != null) 'offset': offset.toString(),
        if (search != null && search!.isNotEmpty) 'search': search,
        if (severity != null) 'severity': severity,
        if (since != null) 'since': since!.toIso8601String(),
        if (until != null) 'until': until!.toIso8601String(),
        if (traceId != null) 'trace_id': traceId,
        if (conversation != null) 'conversation': conversation,
      };
}

/// API endpoint client for log entry retrieval.
///
/// Wraps `GET /api/logs`.
class LogsEndpoint {
  const LogsEndpoint(this._client);

  final AssistantClient _client;

  /// `GET /api/logs` — list recent log entries, newest first.
  Future<List<LogEntry>> list({LogFilters? filters}) async {
    final response = await _client.get(
      '/api/logs',
      queryParams: filters?.toQueryParams(),
    );
    if (response.statusCode != 200) {
      throw ApiException(response.statusCode, response.body);
    }
    final list = jsonDecode(response.body) as List<dynamic>;
    return list
        .cast<Map<String, dynamic>>()
        .map(LogEntry.fromJson)
        .toList();
  }
}
