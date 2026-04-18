import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for TracesApi
void main() {
  final instance = AssistantApi().getTracesApi();

  group(TracesApi, () {
    // `GET /api/traces/{trace_id}` — get a single trace with span breakdown.
    //
    //Future<TraceDetailResponse> getTrace(String traceId) async
    test('test getTrace', () async {
      // TODO
    });

    // `GET /api/traces` — list recent traces, newest first.
    //
    //Future<BuiltList<TraceSummaryResponse>> listTraces({ int limit, int offset, DateTime since, DateTime until, String skill, String status, String conversation }) async
    test('test listTraces', () async {
      // TODO
    });
  });
}
