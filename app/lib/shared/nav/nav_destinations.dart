import 'package:material_ui/material_ui.dart';

/// Navigation destination model.
class NavDest {
  const NavDest({
    required this.path,
    required this.icon,
    required this.selectedIcon,
    required this.label,
  });

  final String path;
  final IconData icon;
  final IconData selectedIcon;
  final String label;
}

// -- Primary destinations (always visible on mobile bottom bar) --

const List<NavDest> primaryDestinations = [
  NavDest(
    path: '/chat',
    icon: Icons.chat_bubble_outline,
    selectedIcon: Icons.chat_bubble,
    label: 'Chat',
  ),
  NavDest(
    path: '/skills',
    icon: Icons.extension_outlined,
    selectedIcon: Icons.extension,
    label: 'Skills',
  ),
  NavDest(
    path: '/workflows',
    icon: Icons.account_tree_outlined,
    selectedIcon: Icons.account_tree,
    label: 'Workflows',
  ),
];

// Context switcher — exposed via trailing slot on desktop, More sheet on mobile.
const NavDest contextsDestination = NavDest(
  path: '/contexts',
  icon: Icons.swap_horiz_outlined,
  selectedIcon: Icons.swap_horiz,
  label: 'Contexts',
);

// -- Overflow destinations (in "More" sheet on mobile; below divider on desktop rail) --

const List<NavDest> overflowDestinations = [
  NavDest(
    path: '/personas',
    icon: Icons.person_outline,
    selectedIcon: Icons.person,
    label: 'Personas',
  ),
  NavDest(
    path: '/webhooks',
    icon: Icons.webhook_outlined,
    selectedIcon: Icons.webhook,
    label: 'Webhooks',
  ),
  NavDest(
    path: '/agents',
    icon: Icons.smart_toy_outlined,
    selectedIcon: Icons.smart_toy,
    label: 'Agents',
  ),
  NavDest(
    path: '/integrations',
    icon: Icons.hub_outlined,
    selectedIcon: Icons.hub,
    label: 'Integrations',
  ),
  NavDest(
    path: '/admin',
    icon: Icons.admin_panel_settings_outlined,
    selectedIcon: Icons.admin_panel_settings,
    label: 'Admin',
  ),
];

// Settings + Contexts switcher — sticky trailing slot on desktop, More sheet on mobile.
const NavDest settingsDestination = NavDest(
  path: '/settings',
  icon: Icons.settings_outlined,
  selectedIcon: Icons.settings,
  label: 'Settings',
);

/// Tab bar destinations used on iOS: primary + Contexts promoted to a tab.
final List<NavDest> iosTabDestinations = [
  primaryDestinations[0], // Chat
  contextsDestination, // Contexts (promoted from overflow)
  primaryDestinations[1], // Skills
  primaryDestinations[2], // Workflows
];

/// Flat list of all destinations shown in the Apple sidebar, in the order
/// CupertinoSidebar assigns indices: primary, then overflow (inside the
/// SidebarSection), then settings.
final List<NavDest> appleSidebarDestinations = [
  ...iosTabDestinations,
  ...overflowDestinations,
  settingsDestination,
];

/// Returns true when [location] belongs to any overflow destination.
bool isOverflowRoute(String location) =>
    overflowDestinations.any((d) => location.startsWith(d.path));

/// Index for the mobile [NavigationBar].
///
/// Primary destinations occupy indices 0–2; the next index is the "More" item.
/// Contexts, Settings and every overflow destination map to "More".
int mobileSelectedIndex(String location) {
  for (var i = 0; i < primaryDestinations.length; i++) {
    if (location.startsWith(primaryDestinations[i].path)) return i;
  }
  if (location.startsWith(contextsDestination.path) ||
      location.startsWith(settingsDestination.path) ||
      isOverflowRoute(location)) {
    return primaryDestinations.length;
  }
  return 0;
}

/// Index for the iOS [CupertinoTabBar].
///
/// iOS tab destinations: Chat(0), Contexts(1), Skills(2), Workflows(3).
/// Index 4 is the "More" item for settings and overflow destinations.
int iosMobileSelectedIndex(String location) {
  for (var i = 0; i < iosTabDestinations.length; i++) {
    if (location.startsWith(iosTabDestinations[i].path)) return i;
  }
  if (location.startsWith(settingsDestination.path) ||
      isOverflowRoute(location)) {
    return iosTabDestinations.length;
  }
  return 0;
}

/// Index for the Apple sidebar: position in [appleSidebarDestinations].
int appleSidebarSelectedIndex(String location) {
  for (var i = 0; i < appleSidebarDestinations.length; i++) {
    if (location.startsWith(appleSidebarDestinations[i].path)) return i;
  }
  return 0;
}

/// Index for the Material collapsible sidebar.
///
/// Maps to the flat list: primary destinations then overflow destinations.
/// Settings and Contexts are in the trailing section, not indexed (-1).
int materialSidebarSelectedIndex(String location) {
  for (var i = 0; i < primaryDestinations.length; i++) {
    if (location.startsWith(primaryDestinations[i].path)) return i;
  }
  for (var i = 0; i < overflowDestinations.length; i++) {
    if (location.startsWith(overflowDestinations[i].path)) {
      return primaryDestinations.length + i;
    }
  }
  return -1;
}
