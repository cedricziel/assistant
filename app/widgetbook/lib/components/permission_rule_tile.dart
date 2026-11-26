import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

final _created = DateTime.utc(2026, 1, 15, 12);

Widget _tile(
  BuildContext context, {
  required PermissionRuleEffect effect,
  String? ownerLabel,
}) => PermissionRuleTile(
  effect: effect,
  rule: context.knobs.string(label: 'rule', initialValue: 'bash(git status*)'),
  createdAt: _created,
  ownerLabel: ownerLabel,
  revoking: context.knobs.boolean(label: 'revoking'),
  onRevoke: () {},
);

final permissionRuleTileComponent = WidgetbookComponent(
  name: 'PermissionRuleTile',
  useCases: [
    WidgetbookUseCase(
      name: 'allow',
      builder: (context) => _tile(context, effect: PermissionRuleEffect.allow),
    ),
    WidgetbookUseCase(
      name: 'deny',
      builder: (context) => _tile(context, effect: PermissionRuleEffect.deny),
    ),
    WidgetbookUseCase(
      name: 'with owner (admin view)',
      builder: (context) => _tile(
        context,
        effect: PermissionRuleEffect.allow,
        ownerLabel: context.knobs.string(
          label: 'owner',
          initialValue: 'ada@example.com',
        ),
      ),
    ),
  ],
);
