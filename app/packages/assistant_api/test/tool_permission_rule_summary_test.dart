import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for ToolPermissionRuleSummary
void main() {
  final instance = ToolPermissionRuleSummaryBuilder();
  // TODO add properties to the builder and call build()

  group(ToolPermissionRuleSummary, () {
    // RFC 3339 creation time.
    // String createdAt
    test('to test the property `createdAt`', () async {
      // TODO
    });

    // `allow` or `deny`.
    // String effect
    test('to test the property `effect`', () async {
      // TODO
    });

    // String id
    test('to test the property `id`', () async {
      // TODO
    });

    // Rule text, for example `bash(git status*)`.
    // String rule
    test('to test the property `rule`', () async {
      // TODO
    });

    // The tool part of the rule, for example `bash`.
    // String toolName
    test('to test the property `toolName`', () async {
      // TODO
    });

    // The user the rule belongs to.
    // String userId
    test('to test the property `userId`', () async {
      // TODO
    });
  });
}
