import 'package:assistant_app/features/pwa/pwa_provider.dart';
import 'package:assistant_app/shared/nav_shell.dart';
import 'package:assistant_app/shared/platform/adaptive_app.dart';
import 'package:cupertino_sidebar/cupertino_sidebar.dart';
import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

/// Pins NavShell's destination selection, overflow sheets and install prompts
/// across the Material and Apple layouts so the widget can be split without
/// visual or behavioural change.

class _PwaNotifier extends PwaInstallNotifier {
  _PwaNotifier(this._installable);
  final bool _installable;

  @override
  bool build() => _installable;
}

const _routes = [
  '/chat',
  '/contexts',
  '/skills',
  '/workflows',
  '/personas',
  '/webhooks',
  '/agents',
  '/admin',
  '/settings',
  '/spaces',
  '/login',
];

Widget _app({
  required String location,
  bool installable = false,
  bool safariInstallable = false,
}) {
  final router = GoRouter(
    initialLocation: location,
    routes: [
      ShellRoute(
        builder: (_, _, child) => NavShell(child: child),
        routes: [
          for (final path in _routes)
            GoRoute(
              path: path,
              builder: (_, _) => Scaffold(body: Text('page:$path')),
            ),
        ],
      ),
    ],
  );
  return ProviderScope(
    overrides: [
      pwaInstallProvider.overrideWith(() => _PwaNotifier(installable)),
      safariInstallProvider.overrideWithValue(safariInstallable),
    ],
    child: MaterialApp.router(
      routerConfig: router,
      builder: (_, child) => LegacyDesignBridge(child: child!),
    ),
  );
}

Future<void> _pump(
  WidgetTester tester,
  Widget app, {
  required Size size,
  TargetPlatform? platform,
}) async {
  debugDefaultTargetPlatformOverride = platform;
  addTearDown(() => debugDefaultTargetPlatformOverride = null);
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(app);
  await tester.pumpAndSettle();
}

const _phone = Size(375, 800);
const _desktop = Size(1280, 900);

/// Labels of the Material sidebar tiles currently marked selected.
List<String> _selectedSidebarLabels(WidgetTester tester) => [
  for (final tile in tester.widgetList<ListTile>(find.byType(ListTile)))
    if (tile.selected) (tile.title! as Text).data!,
];

void main() {
  group('Material mobile — selected tab', () {
    const expected = {
      '/chat': 0,
      '/skills': 1,
      '/workflows': 2,
      '/contexts': 3,
      '/personas': 3,
      '/webhooks': 3,
      '/agents': 3,
      '/admin': 3,
      '/settings': 3,
    };
    for (final entry in expected.entries) {
      testWidgets('${entry.key} selects tab ${entry.value}', (tester) async {
        await _pump(tester, _app(location: entry.key), size: _phone);
        final bar = tester.widget<NavigationBar>(find.byType(NavigationBar));
        expect(bar.selectedIndex, entry.value);
      });
    }

    testWidgets('an unknown route falls back to the first tab', (tester) async {
      await _pump(tester, _app(location: '/login'), size: _phone);
      final bar = tester.widget<NavigationBar>(find.byType(NavigationBar));
      expect(bar.selectedIndex, 0);
    });
  });

  group('Material mobile — More sheet', () {
    testWidgets('lists every destination plus space switch and log out', (
      tester,
    ) async {
      await _pump(tester, _app(location: '/chat'), size: _phone);
      await tester.tap(find.text('More'));
      await tester.pumpAndSettle();

      for (final label in [
        'Switch space',
        'Contexts',
        'Personas',
        'Webhooks',
        'Agents',
        'Admin',
        'Settings',
        'Log out',
      ]) {
        expect(find.text(label), findsOneWidget, reason: label);
      }
      expect(find.byKey(const Key('material-more-logout')), findsOneWidget);
      expect(find.bySemanticsLabel('More destinations'), findsOneWidget);
    });

    testWidgets('tapping Settings closes the sheet and navigates', (
      tester,
    ) async {
      await _pump(tester, _app(location: '/chat'), size: _phone);
      await tester.tap(find.text('More'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Settings'));
      await tester.pumpAndSettle();

      expect(find.text('page:/settings'), findsOneWidget);
      expect(find.text('Switch space'), findsNothing);
    });

    testWidgets('tapping Switch space goes to the space selector', (
      tester,
    ) async {
      await _pump(tester, _app(location: '/chat'), size: _phone);
      await tester.tap(find.text('More'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Switch space'));
      await tester.pumpAndSettle();

      expect(find.text('page:/spaces'), findsOneWidget);
    });
  });

  group('Material desktop — sidebar selection', () {
    const expected = {
      '/chat': 'Chat',
      '/skills': 'Skills',
      '/workflows': 'Workflows',
      '/personas': 'Personas',
      '/webhooks': 'Webhooks',
      '/agents': 'Agents',
      '/admin': 'Admin',
      '/settings': 'Settings',
    };
    for (final entry in expected.entries) {
      testWidgets('${entry.key} highlights only ${entry.value}', (
        tester,
      ) async {
        await _pump(tester, _app(location: entry.key), size: _desktop);
        expect(_selectedSidebarLabels(tester), [entry.value]);
      });
    }

    testWidgets('/contexts highlights Contexts', (tester) async {
      await _pump(tester, _app(location: '/contexts'), size: _desktop);
      expect(_selectedSidebarLabels(tester), ['Contexts']);
    });

    testWidgets('tapping a sidebar item navigates', (tester) async {
      await _pump(tester, _app(location: '/chat'), size: _desktop);
      await tester.tap(find.text('Agents'));
      await tester.pumpAndSettle();
      expect(find.text('page:/agents'), findsOneWidget);
    });
  });

  group('Install prompts', () {
    testWidgets('mobile shows the install banner above the tab bar', (
      tester,
    ) async {
      await _pump(
        tester,
        _app(location: '/chat', installable: true),
        size: _phone,
      );
      final banner = find.text('Install App');
      expect(banner, findsOneWidget);
      expect(
        tester.getBottomLeft(banner).dy,
        lessThan(tester.getTopLeft(find.byType(NavigationBar)).dy),
      );
    });

    testWidgets('mobile Safari banner opens the add-to-home-screen steps', (
      tester,
    ) async {
      await _pump(
        tester,
        _app(location: '/chat', safariInstallable: true),
        size: _phone,
      );
      await tester.tap(find.text('Add to Home Screen'));
      await tester.pumpAndSettle();

      expect(
        find.text('Tap the Share button in the Safari toolbar.'),
        findsOneWidget,
      );
      expect(
        find.text('Scroll down and tap "Add to Home Screen".'),
        findsOneWidget,
      );
      expect(find.text('Tap "Add" to confirm.'), findsOneWidget);
    });

    testWidgets('mobile without prompts shows no install affordance', (
      tester,
    ) async {
      await _pump(tester, _app(location: '/chat'), size: _phone);
      expect(find.text('Install App'), findsNothing);
      expect(find.text('Add to Home Screen'), findsNothing);
    });

    testWidgets('desktop shows install buttons in the sidebar', (tester) async {
      await _pump(
        tester,
        _app(location: '/chat', installable: true, safariInstallable: true),
        size: _desktop,
      );
      expect(find.byTooltip('Install App'), findsOneWidget);
      expect(find.byTooltip('Add to Home Screen'), findsOneWidget);
    });
  });

  group('Apple compact — tab bar', () {
    const expected = {
      '/chat': 0,
      '/contexts': 1,
      '/skills': 2,
      '/workflows': 3,
      '/personas': 4,
      '/agents': 4,
      '/settings': 4,
    };
    for (final entry in expected.entries) {
      testWidgets('${entry.key} selects tab ${entry.value}', (tester) async {
        await _pump(
          tester,
          _app(location: entry.key),
          size: _phone,
          platform: TargetPlatform.iOS,
        );
        final bar = tester.widget<CupertinoTabBar>(
          find.byType(CupertinoTabBar),
        );
        expect(bar.currentIndex, entry.value);
      });
    }

    testWidgets('More sheet offers space switch, settings, log out, cancel', (
      tester,
    ) async {
      await _pump(
        tester,
        _app(location: '/chat'),
        size: _phone,
        platform: TargetPlatform.iOS,
      );
      await tester.tap(find.text('More'));
      await tester.pumpAndSettle();

      for (final label in ['Switch space', 'Settings', 'Log out', 'Cancel']) {
        expect(find.text(label), findsOneWidget, reason: label);
      }
      expect(find.byKey(const Key('cupertino-more-logout')), findsOneWidget);

      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(find.byType(CupertinoActionSheet), findsNothing);
    });
  });

  group('Apple wide — sidebar', () {
    const expected = {
      '/chat': 0,
      '/contexts': 1,
      '/skills': 2,
      '/workflows': 3,
      '/personas': 4,
      '/webhooks': 5,
      '/agents': 6,
      '/admin': 7,
      '/settings': 8,
    };
    for (final entry in expected.entries) {
      testWidgets('${entry.key} selects sidebar row ${entry.value}', (
        tester,
      ) async {
        await _pump(
          tester,
          _app(location: entry.key),
          size: const Size(1024, 768),
          platform: TargetPlatform.iOS,
        );
        final sidebar = tester.widget<CupertinoSidebar>(
          find.byType(CupertinoSidebar),
        );
        expect(sidebar.selectedIndex, entry.value);
      });
    }

    testWidgets('shows the toggle, the logout button and the space title', (
      tester,
    ) async {
      await _pump(
        tester,
        _app(location: '/chat'),
        size: const Size(1024, 768),
        platform: TargetPlatform.iOS,
      );
      expect(find.bySemanticsLabel('Collapse sidebar'), findsOneWidget);
      expect(find.byKey(const Key('cupertino-logout')), findsOneWidget);
      expect(find.text('Spaces'), findsOneWidget);
    });

    testWidgets('selecting a sidebar row navigates', (tester) async {
      await _pump(
        tester,
        _app(location: '/chat'),
        size: const Size(1024, 768),
        platform: TargetPlatform.iOS,
      );
      await tester.tap(find.text('Agents'));
      await tester.pumpAndSettle();
      expect(find.text('page:/agents'), findsOneWidget);
    });
  });
}
