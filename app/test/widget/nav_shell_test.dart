import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/api/models/server_profile.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/notifications/notification_preferences.dart';
import 'package:assistant_app/features/notifications/notification_service.dart';
import 'package:assistant_app/features/pwa/pwa_provider.dart';
import 'package:assistant_app/features/updater/update_provider.dart';
import 'package:assistant_app/features/updater/update_service.dart';
import 'package:assistant_app/shared/nav_shell.dart';

// ---------------------------------------------------------------------------
// Fakes

class _FakeServerProfileNotifier extends ServerProfileNotifier {
  _FakeServerProfileNotifier(this._profile);
  final ServerProfile? _profile;

  @override
  Future<ServerConnectionState> build() async =>
      ServerConnectionState(profile: _profile);
}

class _FakePwaNotifier extends PwaInstallNotifier {
  _FakePwaNotifier(this._installable);
  final bool _installable;

  @override
  bool build() => _installable;

  @override
  void install() {}
}

class _FakeChatNotifier extends ChatNotifier {
  @override
  Future<ChatState> build() async => const ChatState();
}

class _FakeUpdateNotifier extends UpdateCheckNotifier {
  @override
  Future<UpdateInfo?> build() async => null;
}

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

class _SyncPrefsNotifier extends NotificationPreferencesNotifier {
  @override
  Future<NotificationPreferences> build() async {
    final prefs = await SharedPreferences.getInstance();
    return NotificationPreferences(prefs);
  }
}

// ---------------------------------------------------------------------------
// Helpers

final _allPaths = [
  '/chat',
  '/traces',
  '/logs',
  '/contexts',
  '/skills',
  '/workflows',
  '/webhooks',
  '/agents',
  '/analytics',
  '/settings',
];

GoRouter _buildRouter() => GoRouter(
  initialLocation: '/chat',
  routes: [
    ShellRoute(
      builder: (context, state, child) => NavShell(child: child),
      routes: [
        for (final path in _allPaths)
          GoRoute(
            path: path,
            builder: (context, state) =>
                Scaffold(body: Text(path.replaceAll('/', ''))),
          ),
      ],
    ),
  ],
);

Widget _buildApp({bool pwaInstallable = false, ServerProfile? serverProfile}) {
  final profile =
      serverProfile ??
      const ServerProfile(
        baseUrl: 'http://localhost:8080',
        token: 't',
        label: 'Local',
      );
  return ProviderScope(
    overrides: [
      serverProfileProvider.overrideWith(
        () => _FakeServerProfileNotifier(profile),
      ),
      pwaInstallProvider.overrideWith(() => _FakePwaNotifier(pwaInstallable)),
      chatProvider.overrideWith(_FakeChatNotifier.new),
      updateCheckProvider.overrideWith(_FakeUpdateNotifier.new),
      notificationServiceProvider.overrideWith(
        (ref) => _FakeNotificationService(ref: ref),
      ),
      notificationPreferencesProvider.overrideWith(_SyncPrefsNotifier.new),
    ],
    child: MaterialApp.router(routerConfig: _buildRouter()),
  );
}

// ---------------------------------------------------------------------------

void main() {
  setUp(() {
    SharedPreferences.setMockInitialValues({});
  });

  group('NavShell – narrow layout (< 768 px)', () {
    setUp(() {});

    testWidgets('does not show a NavigationRail', (tester) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      expect(find.byType(NavigationRail), findsNothing);
    });

    testWidgets('does not show a bottom navigation bar', (tester) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      expect(find.byType(NavigationBar), findsNothing);
      expect(find.byType(BottomNavigationBar), findsNothing);
    });

    testWidgets('scaffold has a drawer', (tester) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      final scaffold = tester.widget<Scaffold>(find.byType(Scaffold).first);
      expect(
        scaffold.drawer,
        isNotNull,
        reason: 'Scaffold should have a drawer on narrow screens',
      );
    });

    testWidgets('AppBar is visible with a drawer opener', (tester) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      expect(find.byType(AppBar), findsOneWidget);
      // DrawerButton or a menu icon should be present in the AppBar.
      expect(
        find.byType(DrawerButton).evaluate().isNotEmpty ||
            find.byIcon(Icons.menu).evaluate().isNotEmpty,
        isTrue,
        reason:
            'AppBar should contain a drawer opener (DrawerButton or menu icon)',
      );
    });

    testWidgets('drawer contains all primary destinations when opened', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      // Open the drawer.
      final scaffoldState = tester.state<ScaffoldState>(
        find.byType(Scaffold).first,
      );
      scaffoldState.openDrawer();
      await tester.pumpAndSettle();

      for (final label in [
        'Chat',
        'Traces',
        'Logs',
        'Personas',
        'Skills',
        'Workflows',
        'Webhooks',
        'Agents',
        'Analytics',
      ]) {
        expect(
          find.text(label),
          findsWidgets,
          reason: '$label destination should be in the drawer',
        );
      }
    });

    testWidgets('drawer contains Settings in a separate section when opened', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      final scaffoldState = tester.state<ScaffoldState>(
        find.byType(Scaffold).first,
      );
      scaffoldState.openDrawer();
      await tester.pumpAndSettle();

      expect(
        find.text('Settings'),
        findsWidgets,
        reason: 'Settings should be present in the drawer',
      );
      // A Divider should visually separate Settings from primary destinations.
      expect(
        find.byType(Divider),
        findsWidgets,
        reason: 'Drawer should have a Divider separating Settings',
      );
    });

    testWidgets('drawer shows PWA install option when installable', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp(pwaInstallable: true));
      await tester.pump();

      final scaffoldState = tester.state<ScaffoldState>(
        find.byType(Scaffold).first,
      );
      scaffoldState.openDrawer();
      await tester.pumpAndSettle();

      expect(
        find.text('Install App'),
        findsOneWidget,
        reason:
            'Install App option should appear in the drawer when installable',
      );
    });

    testWidgets(
      'drawer does not show PWA install option when not installable',
      (tester) async {
        tester.view.physicalSize = const Size(400, 800);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.resetPhysicalSize);

        await tester.pumpWidget(_buildApp(pwaInstallable: false));
        await tester.pump();

        final scaffoldState = tester.state<ScaffoldState>(
          find.byType(Scaffold).first,
        );
        scaffoldState.openDrawer();
        await tester.pumpAndSettle();

        expect(
          find.text('Install App'),
          findsNothing,
          reason: 'Install App option should not appear when not installable',
        );
      },
    );

    testWidgets('tapping a destination closes the drawer', (tester) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      final scaffoldState = tester.state<ScaffoldState>(
        find.byType(Scaffold).first,
      );
      scaffoldState.openDrawer();
      await tester.pumpAndSettle();

      await tester.tap(find.text('Traces').first);
      await tester.pumpAndSettle();

      // Drawer should be closed after navigation.
      expect(
        scaffoldState.isDrawerOpen,
        isFalse,
        reason: 'Drawer should close after tapping a destination',
      );
    });

    testWidgets('drawer header shows server displayLabel', (tester) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      final scaffoldState = tester.state<ScaffoldState>(
        find.byType(Scaffold).first,
      );
      scaffoldState.openDrawer();
      await tester.pumpAndSettle();

      expect(
        find.text('Local'),
        findsWidgets,
        reason: 'Drawer header should show the server displayLabel',
      );
    });
  });

  // -------------------------------------------------------------------------

  group('NavShell – wide layout (>= 768 px)', () {
    testWidgets('shows a NavigationRail', (tester) async {
      tester.view.physicalSize = const Size(1024, 768);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      expect(find.byType(NavigationRail), findsOneWidget);
    });

    testWidgets('does not show a bottom navigation bar', (tester) async {
      tester.view.physicalSize = const Size(1024, 768);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      expect(find.byType(NavigationBar), findsNothing);
      expect(find.byType(BottomNavigationBar), findsNothing);
    });

    testWidgets('NavigationRail trailing shows install button when installable', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1024, 768);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp(pwaInstallable: true));
      await tester.pump();

      // The install button tooltip or icon should be present.
      expect(
        find.byTooltip('Install App').evaluate().isNotEmpty ||
            find.byIcon(Icons.install_mobile).evaluate().isNotEmpty,
        isTrue,
        reason:
            'Install button should appear in NavigationRail trailing when installable',
      );
    });

    testWidgets(
      'NavigationRail trailing has no install button when not installable',
      (tester) async {
        tester.view.physicalSize = const Size(1024, 768);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.resetPhysicalSize);

        await tester.pumpWidget(_buildApp(pwaInstallable: false));
        await tester.pump();

        expect(find.byIcon(Icons.install_mobile), findsNothing);
      },
    );

    testWidgets('Settings is not a NavigationRail destination', (tester) async {
      tester.view.physicalSize = const Size(1024, 768);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      expect(
        find
            .descendant(
              of: find.byType(NavigationRail),
              matching: find.text('Settings'),
            )
            .evaluate()
            .isEmpty,
        isTrue,
        reason: 'Settings should not be a NavigationRail destination',
      );
    });

    testWidgets('Settings icon button visible in rail trailing', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1024, 768);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      expect(
        find.byIcon(Icons.settings_outlined).evaluate().isNotEmpty ||
            find.byIcon(Icons.settings).evaluate().isNotEmpty,
        isTrue,
        reason: 'Settings icon button should be visible in rail trailing',
      );
    });

    testWidgets('server chip visible in rail trailing showing displayLabel', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1024, 768);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      expect(
        find.text('Local'),
        findsOneWidget,
        reason: 'ServerChip in rail trailing should show displayLabel',
      );
    });

    testWidgets('tapping rail Settings icon navigates to settings', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1024, 768);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(_buildApp());
      await tester.pump();

      final settingsIcon = find.byTooltip('Settings');
      expect(settingsIcon, findsOneWidget);

      await tester.tap(settingsIcon);
      await tester.pumpAndSettle();

      // No crash — navigation completes.
      expect(find.byType(NavigationRail), findsOneWidget);
    });
  });
}
