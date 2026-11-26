@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

void main() {
  final cases = <String, ({PermissionRuleEffect effect, String? owner})>{
    'permission_rule_tile_allow': (
      effect: PermissionRuleEffect.allow,
      owner: null,
    ),
    'permission_rule_tile_deny': (
      effect: PermissionRuleEffect.deny,
      owner: null,
    ),
    'permission_rule_tile_owner': (
      effect: PermissionRuleEffect.allow,
      owner: 'ada@example.com',
    ),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          Padding(
            padding: const EdgeInsets.all(16),
            child: PermissionRuleTile(
              effect: entry.value.effect,
              rule: 'bash(git status*)',
              createdAt: kGoldenNow,
              ownerLabel: entry.value.owner,
              onRevoke: () {},
            ),
          ),
          size: const Size(400, 120),
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 50));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
