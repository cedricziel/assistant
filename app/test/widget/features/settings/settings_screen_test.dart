import 'package:assistant_api/assistant_api.dart'
    show ToolPermissionRuleSummary;
import 'package:assistant_app/features/notifications/notification_service.dart';
import 'package:assistant_app/features/settings/response_timing_preference.dart';
import 'package:assistant_app/features/settings/settings_screen.dart';
import 'package:assistant_app/features/settings/tool_approvals_section.dart';
import 'package:assistant_app/features/settings/tool_permission_rules_provider.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  setUp(() {
    SharedPreferences.setMockInitialValues({});
  });

  Widget buildUnderTest() {
    return ProviderScope(
      overrides: [
        // Stub out NotificationService so it never calls the real plugin.
        notificationServiceProvider.overrideWith(
          (ref) => _FakeNotificationService(ref: ref),
        ),
        toolPermissionRulesProvider.overrideWith(_NoRulesNotifier.new),
      ],
      child: const MaterialApp(home: SettingsScreen()),
    );
  }

  group('SettingsScreen – Notifications section', () {
    testWidgets('renders all four notification toggles', (tester) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pump(); // let async provider load

      expect(find.text('New chat messages'), findsOneWidget);
      expect(find.text('Skill completions'), findsOneWidget);
      expect(find.text('Agent errors'), findsOneWidget);
      expect(find.text('Approval requests'), findsOneWidget);
    });

    testWidgets('toggles are on by default', (tester) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pump();

      for (final title in [
        'New chat messages',
        'Skill completions',
        'Agent errors',
        'Approval requests',
      ]) {
        final sw = tester.widget<Switch>(
          find.descendant(
            of: find.ancestor(
              of: find.text(title),
              matching: find.byType(SwitchListTile),
            ),
            matching: find.byType(Switch),
          ),
        );
        expect(sw.value, isTrue, reason: '$title should default to true');
      }
    });

    testWidgets('tapping "New chat messages" toggle turns it off', (
      tester,
    ) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pump();

      final chatTile = find.ancestor(
        of: find.text('New chat messages'),
        matching: find.byType(SwitchListTile),
      );
      await tester.tap(chatTile);
      await tester.pump();

      // Re-read the persisted value.
      final prefs = await SharedPreferences.getInstance();
      expect(prefs.getBool('notifications.chat_messages'), isFalse);
    });

    testWidgets('tapping "Approval requests" turns it off and persists', (
      tester,
    ) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pump();

      final tile = find.ancestor(
        of: find.text('Approval requests'),
        matching: find.byType(SwitchListTile),
      );
      await tester.ensureVisible(tile);
      await tester.pumpAndSettle();
      await tester.tap(tile);
      await tester.pump();

      final prefs = await SharedPreferences.getInstance();
      expect(prefs.getBool('notifications.approval_requests'), isFalse);
    });
  });

  group('SettingsScreen – Tool approvals', () {
    testWidgets('shows the Tool approvals section', (tester) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pump();

      await tester.scrollUntilVisible(
        find.byType(ToolApprovalsSection),
        200,
        scrollable: find.byType(Scrollable).first,
      );

      expect(find.byType(ToolApprovalsSection), findsOneWidget);
      expect(find.text('Tool approvals'), findsOneWidget);
    });
  });

  group('SettingsScreen – Show response timing', () {
    final timingTile = find.ancestor(
      of: find.text('Show response timing'),
      matching: find.byType(SwitchListTile),
    );

    ProviderContainer containerOf(WidgetTester tester) =>
        ProviderScope.containerOf(tester.element(find.byType(SettingsScreen)));

    testWidgets('shows the switch row with its subtitle, off by default', (
      tester,
    ) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pump();

      expect(timingTile, findsOneWidget);
      expect(
        find.text(
          'Show first-token time, total time and tokens per second under '
          'finished replies.',
        ),
        findsOneWidget,
      );
      expect(tester.widget<SwitchListTile>(timingTile).value, isFalse);
      expect(containerOf(tester).read(showResponseTimingProvider).value, false);
    });

    testWidgets('tapping the row turns the provider on', (tester) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pump();

      await tester.tap(timingTile);
      await tester.pump();

      expect(containerOf(tester).read(showResponseTimingProvider).value, true);
      expect(tester.widget<SwitchListTile>(timingTile).value, isTrue);
    });

    testWidgets('is a focusable, tappable node labelled with the title', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      await tester.pumpWidget(buildUnderTest());
      await tester.pump();

      final node = tester.getSemantics(timingTile);
      expect(node.label, startsWith('Show response timing'));
      expect(
        node,
        matchesSemantics(
          label: node.label,
          hasEnabledState: true,
          isEnabled: true,
          isFocusable: true,
          hasSelectedState: true,
          hasToggledState: true,
          hasTapAction: true,
          hasFocusAction: true,
        ),
      );
      handle.dispose();
    });

    for (final key in [LogicalKeyboardKey.space, LogicalKeyboardKey.enter]) {
      testWidgets('toggles on ${key.debugName}', (tester) async {
        await tester.pumpWidget(buildUnderTest());
        await tester.pump();

        // Tab until the focused node sits inside the timing row.
        bool timingRowFocused() {
          final tile = FocusManager.instance.primaryFocus?.context
              ?.findAncestorWidgetOfExactType<SwitchListTile>();
          final title = tile?.title;
          return title is Text && title.data == 'Show response timing';
        }

        for (var i = 0; i < 10 && !timingRowFocused(); i++) {
          await tester.sendKeyEvent(LogicalKeyboardKey.tab);
          await tester.pump();
        }
        expect(timingRowFocused(), isTrue, reason: 'row must be reachable');

        await tester.sendKeyEvent(key);
        await tester.pump();

        expect(
          containerOf(tester).read(showResponseTimingProvider).value,
          true,
        );
      });
    }
  });
}

class _NoRulesNotifier extends ToolPermissionRulesNotifier {
  @override
  Future<List<ToolPermissionRuleSummary>> build() async => const [];
}

// ---------------------------------------------------------------------------
// Fake NotificationService that never calls the real plugin.

class _FakeNotificationService extends NotificationService {
  _FakeNotificationService({required super.ref});

  @override
  Future<void> initialize() async {}

  @override
  Future<bool> requestPermission() async => true;

  @override
  Future<void> show(
    String title,
    String body, {
    String? conversationId,
  }) async {}
}
