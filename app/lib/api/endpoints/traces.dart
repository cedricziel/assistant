import 'dart:convert';

import '../client.dart';
import '../models/trace.dart';
import 'conversations.dart';

/// Filter parameters for `GET /api/traces`.
class TraceFilters {
  const TraceFilters({
    this.limit,
    this.offset,
    this.since,
    this.until,
    this.skill,
    this.status,
    this.conversation,
  });

  final int? limit;
  final int? offset;
  final DateTime? since;
  final DateTime? until;
  final String? skill;

  /// `"ok"` or `"error"`.
  final String? status;
  final String? conversation;

  Map<String, String?> toQueryParams() => {
        if (limit != null) 'limit': limit.toString(),
        if (offset != null) 'offset': offset.toString(),
        if (since != null) 'since': since!.toIso8601String(),
        if (until != null) 'until': until!.toIso8601String(),
        if (skill != null) 'skill': skill,
        if (status != null) 'status': status,
        if (conversation != null) 'conversation': conversation,
      };
}

/// API endpoint client for observability traces.
///
/// Wraps `GET /api/traces` and `GET /api/traces/{trace_id}`.
class TracesEndpoint {
  const TracesEndpoint(this._client);

  final AssistantClient _client;

  /// `GET /api/traces` — list recent traces, newest first.
  Future<List<TraceSummary>> list({TraceFilters? filters}) async {
    final response = await _client.get(
      '/api/traces',
      queryParams: filters?.toQueryParams(),
    );
    if (response.statusCode != 200) {
      throw ApiException(response.statusCode, response.body);
    }
    final list = jsonDecode(response.body) as List<dynamic>;
    return list
        .cast<Map<String, dynamic>>()
        .map(TraceSummary.fromJson)
        .toList();
  }

  /// `GET /api/traces/{traceId}` — get a single trace with span breakdown.
  Future<TraceDetail> get(String traceId) async {
    final response = await _client.get('/api/traces/$traceId');
    if (response.statusCode == 404) {
      throw NotFoundException('Trace not found: $traceId');
    }
    if (response.statusCode != 200) {
      throw ApiException(response.statusCode, response.body);
    }
    return TraceDetail.fromJson(
      jsonDecode(response.body) as Map<String, dynamic>,
    );
  }
}
