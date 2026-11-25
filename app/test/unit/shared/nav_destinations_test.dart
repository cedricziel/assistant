import 'package:assistant_app/shared/nav/nav_destinations.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('destination tables', () {
    test('primary destinations are Chat, Skills, Workflows', () {
      expect(primaryDestinations.map((d) => d.path), [
        '/chat',
        '/skills',
        '/workflows',
      ]);
    });

    test('overflow destinations keep their order', () {
      expect(overflowDestinations.map((d) => d.label), [
        'Personas',
        'Webhooks',
        'Agents',
        'Integrations',
        'Admin',
      ]);
    });

    test('iOS tabs promote Contexts to second place', () {
      expect(iosTabDestinations.map((d) => d.label), [
        'Chat',
        'Contexts',
        'Skills',
        'Workflows',
      ]);
    });

    test('Apple sidebar lists tabs, overflow, then settings', () {
      expect(appleSidebarDestinations.map((d) => d.label), [
        'Chat',
        'Contexts',
        'Skills',
        'Workflows',
        'Personas',
        'Webhooks',
        'Agents',
        'Integrations',
        'Admin',
        'Settings',
      ]);
    });
  });

  group('isOverflowRoute', () {
    test('matches overflow destinations and their sub-paths', () {
      expect(isOverflowRoute('/webhooks'), isTrue);
      expect(isOverflowRoute('/webhooks/abc'), isTrue);
      expect(isOverflowRoute('/integrations'), isTrue);
      expect(isOverflowRoute('/admin/users'), isTrue);
    });

    test('rejects primary, settings and unknown routes', () {
      expect(isOverflowRoute('/chat'), isFalse);
      expect(isOverflowRoute('/settings'), isFalse);
      expect(isOverflowRoute('/nope'), isFalse);
    });
  });

  group('mobileSelectedIndex', () {
    test('maps primary routes to their tab', () {
      expect(mobileSelectedIndex('/chat/123'), 0);
      expect(mobileSelectedIndex('/skills'), 1);
      expect(mobileSelectedIndex('/workflows/x'), 2);
    });

    test('maps contexts, settings and overflow routes to More', () {
      for (final path in [
        '/contexts',
        '/settings',
        '/personas',
        '/integrations',
        '/admin',
      ]) {
        expect(mobileSelectedIndex(path), 3, reason: path);
      }
    });

    test('falls back to the first tab', () {
      expect(mobileSelectedIndex('/unknown'), 0);
    });
  });

  group('iosMobileSelectedIndex', () {
    test('maps tab routes in tab order', () {
      expect(iosMobileSelectedIndex('/chat'), 0);
      expect(iosMobileSelectedIndex('/contexts'), 1);
      expect(iosMobileSelectedIndex('/skills'), 2);
      expect(iosMobileSelectedIndex('/workflows'), 3);
    });

    test('maps settings and overflow routes to More', () {
      expect(iosMobileSelectedIndex('/settings'), 4);
      expect(iosMobileSelectedIndex('/agents'), 4);
    });

    test('falls back to the first tab', () {
      expect(iosMobileSelectedIndex('/unknown'), 0);
    });
  });

  group('appleSidebarSelectedIndex', () {
    test('indexes into the flat Apple sidebar list', () {
      expect(appleSidebarSelectedIndex('/chat'), 0);
      expect(appleSidebarSelectedIndex('/contexts'), 1);
      expect(appleSidebarSelectedIndex('/integrations'), 7);
      expect(appleSidebarSelectedIndex('/admin'), 8);
      expect(appleSidebarSelectedIndex('/settings'), 9);
    });

    test('falls back to the first row', () {
      expect(appleSidebarSelectedIndex('/unknown'), 0);
    });
  });

  group('materialSidebarSelectedIndex', () {
    test('indexes primary then overflow destinations', () {
      expect(materialSidebarSelectedIndex('/chat'), 0);
      expect(materialSidebarSelectedIndex('/workflows'), 2);
      expect(materialSidebarSelectedIndex('/personas'), 3);
      expect(materialSidebarSelectedIndex('/integrations'), 6);
      expect(materialSidebarSelectedIndex('/admin'), 7);
    });

    test('returns -1 for the trailing section and unknown routes', () {
      expect(materialSidebarSelectedIndex('/settings'), -1);
      expect(materialSidebarSelectedIndex('/contexts'), -1);
      expect(materialSidebarSelectedIndex('/unknown'), -1);
    });
  });
}
