import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for ToolPermissionRulesApi
void main() {
  final instance = AssistantApi().getToolPermissionRulesApi();

  group(ToolPermissionRulesApi, () {
    // `DELETE /api/tool-permission-rules/{id}`
    //
    // Revoke a saved rule. Members can revoke their own rules; a space admin or org admin can revoke any rule in the selected space.
    //
    //Future deleteToolPermissionRule(String id, { String xSpaceId }) async
    test('test deleteToolPermissionRule', () async {
      // TODO
    });

    // `GET /api/tool-permission-rules`
    //
    // The caller's saved rules in the selected space, oldest first. With `scope=space`, every member's rules (space admins and org admins only).
    //
    //Future<BuiltList<ToolPermissionRuleSummary>> listToolPermissionRules({ RuleScope scope, String xSpaceId }) async
    test('test listToolPermissionRules', () async {
      // TODO
    });
  });
}
