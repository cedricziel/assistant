import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';


/// tests for LogsApi
void main() {
  final instance = AssistantApi().getLogsApi();

  group(LogsApi, () {
    // `GET /api/logs` — list recent log entries, newest first.
    //
    //Future<BuiltList<LogEntryResponse>> listLogs({ int limit, int offset, String search, String severity, DateTime since, DateTime until, String traceId, String conversation }) async
    test('test listLogs', () async {
      // TODO
    });

  });
}
