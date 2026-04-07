import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';


/// tests for AnalyticsApi
void main() {
  final instance = AssistantApi().getAnalyticsApi();

  group(AnalyticsApi, () {
    // `GET /api/analytics` — get aggregated usage analytics for a time window.
    //
    //Future<AnalyticsSummaryResponse> getAnalytics({ int window }) async
    test('test getAnalytics', () async {
      // TODO
    });

  });
}
