import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/integrations/integration_list_tile.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../unit/integrations/integration_fakes.dart';

Future<void> _pump(
  WidgetTester tester,
  IntegrationSummary integration, {
  bool busy = false,
  ValueChanged<bool>? onEnabledChanged,
  VoidCallback? onDelete,
  bool optedOut = false,
  ValueChanged<bool>? onOptedOutChanged,
  String? actsAs,
  double width = 600,
}) {
  tester.view.physicalSize = Size(width, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  return tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: IntegrationListTile(
          integration: integration,
          busy: busy,
          onEnabledChanged: onEnabledChanged,
          onDelete: onDelete,
          optedOut: optedOut,
          onOptedOutChanged: onOptedOutChanged,
          actsAs: actsAs,
        ),
      ),
    ),
  );
}

void main() {
  group('IntegrationListTile', () {
    testWidgets('shows name, layer, slug and a status chip', (tester) async {
      await _pump(tester, integrationSummary('int_linear', name: 'Linear'));

      expect(find.text('Linear'), findsOneWidget);
      expect(find.textContaining('Organization'), findsOneWidget);
      expect(find.textContaining('int-linear'), findsOneWidget);
      expect(find.text('Ready'), findsOneWidget);
    });

    for (final (status, label) in [
      (IntegrationStatus.ready, 'Ready'),
      (IntegrationStatus.needsConnection, 'Needs connection'),
      (IntegrationStatus.authExpired, 'Sign-in expired'),
      (IntegrationStatus.unreachable, 'Unreachable'),
      (IntegrationStatus.blockedByPolicy, 'Blocked by policy'),
      (IntegrationStatus.disabled, 'Disabled'),
    ]) {
      testWidgets('labels status ${status.name} as "$label"', (tester) async {
        await _pump(tester, integrationSummary('int_a', status: status));

        expect(find.text(label), findsOneWidget);
      });
    }

    testWidgets('says who connects for per-user and shared credentials', (
      tester,
    ) async {
      await _pump(
        tester,
        integrationSummary('int_a', mode: CredentialMode.perUser),
      );
      expect(
        find.text('Each person connects their own account'),
        findsOneWidget,
      );

      await _pump(tester, integrationSummary('int_a'));
      expect(find.text('One shared credential'), findsOneWidget);
    });

    testWidgets('shows "acts as" only when the account is known', (
      tester,
    ) async {
      await _pump(tester, integrationSummary('int_a'));
      expect(find.textContaining('Acts as'), findsNothing);

      await _pump(
        tester,
        integrationSummary('int_a'),
        actsAs: 'ada@example.com',
      );
      expect(find.text('Acts as ada@example.com'), findsOneWidget);
    });

    testWidgets('has no controls when read-only', (tester) async {
      await _pump(tester, integrationSummary('int_a'));

      expect(find.byType(Switch), findsNothing);
      expect(find.byTooltip('Remove'), findsNothing);
    });

    testWidgets('the enable switch reports changes', (tester) async {
      bool? changed;
      await _pump(
        tester,
        integrationSummary('int_a'),
        onEnabledChanged: (v) => changed = v,
      );

      expect(tester.widget<Switch>(find.byType(Switch)).value, isTrue);
      await tester.tap(find.byType(Switch));

      expect(changed, isFalse);
    });

    testWidgets('a disabled integration shows the switch off', (tester) async {
      await _pump(
        tester,
        integrationSummary('int_a', enabled: false),
        onEnabledChanged: (_) {},
      );

      expect(tester.widget<Switch>(find.byType(Switch)).value, isFalse);
    });

    testWidgets('the opt-out switch is on while the space uses it', (
      tester,
    ) async {
      bool? changed;
      await _pump(
        tester,
        integrationSummary('int_a'),
        onOptedOutChanged: (v) => changed = v,
      );

      expect(find.text('Use in this space'), findsOneWidget);
      expect(tester.widget<Switch>(find.byType(Switch)).value, isTrue);
      await tester.tap(find.byType(Switch));
      expect(changed, isTrue, reason: 'switching off means opting out');
    });

    testWidgets('an opted-out integration shows the switch off', (
      tester,
    ) async {
      await _pump(
        tester,
        integrationSummary('int_a'),
        optedOut: true,
        onOptedOutChanged: (_) {},
      );

      expect(tester.widget<Switch>(find.byType(Switch)).value, isFalse);
    });

    testWidgets('the delete button reports taps', (tester) async {
      var deletes = 0;
      await _pump(
        tester,
        integrationSummary('int_a'),
        onDelete: () => deletes++,
      );

      await tester.tap(find.byTooltip('Remove'));

      expect(deletes, 1);
    });

    testWidgets('controls are disabled while busy', (tester) async {
      await _pump(
        tester,
        integrationSummary('int_a'),
        busy: true,
        onEnabledChanged: (_) {},
        onDelete: () {},
      );

      expect(tester.widget<Switch>(find.byType(Switch)).onChanged, isNull);
      expect(
        tester.widget<IconButton>(find.byType(IconButton)).onPressed,
        isNull,
      );
    });

    testWidgets('needs-connection offers a disabled Connect with a hint', (
      tester,
    ) async {
      await _pump(
        tester,
        integrationSummary('int_a', status: IntegrationStatus.needsConnection),
      );

      final button = find.widgetWithText(FilledButton, 'Connect');
      expect(button, findsOneWidget);
      expect(tester.widget<FilledButton>(button).onPressed, isNull);
      expect(
        find.byTooltip('Credential setup arrives in a later update.'),
        findsOneWidget,
      );
    });

    testWidgets('a ready integration has no Connect button', (tester) async {
      await _pump(tester, integrationSummary('int_a'));

      expect(find.text('Connect'), findsNothing);
    });

    testWidgets('fits a narrow phone without overflow', (tester) async {
      await _pump(
        tester,
        integrationSummary(
          'int_a',
          name: 'A rather long integration name for a phone',
          status: IntegrationStatus.needsConnection,
          mode: CredentialMode.perUser,
        ),
        width: 320,
        onEnabledChanged: (_) {},
        onDelete: () {},
      );

      expect(tester.takeException(), isNull);
    });
  });
}
