import 'package:flutter_riverpod/flutter_riverpod.dart'
    show WidgetRef, AsyncValue, ProviderSubscription;
import 'package:tray_manager/tray_manager.dart';
import 'package:window_manager/window_manager.dart';

import '../features/contexts/models/context_model.dart';
import '../features/contexts/providers/context_providers.dart';

/// Initializes the macOS menu bar tray icon and wires up context-menu actions.
///
/// Call [init] once on startup (macOS only).  Holds a [Ref] so it can read
/// [contextsProvider] and [activeContextProvider] to populate the menu.
class TrayService with TrayListener {
  static final TrayService _instance = TrayService._();
  factory TrayService() => _instance;
  TrayService._();

  WidgetRef? _ref;
  final List<ProviderSubscription<dynamic>> _subscriptions = [];

  /// Sets up the tray icon.  Call this before [runApp] (no [Ref] required).
  Future<void> initIcon() async {
    trayManager.addListener(this);
    await trayManager.setIcon('assets/tray_icon.png');
    // Show a static menu until [initMenu] is called with a Ref.
    await trayManager.setContextMenu(
      Menu(
        items: [
          MenuItem(key: 'show_window', label: 'Open'),
          MenuItem.separator(),
          MenuItem(key: 'quit', label: 'Quit'),
        ],
      ),
    );
  }

  /// Wires reactive menu updates.  Call from the widget tree once [Ref] is
  /// available (e.g. in [ConsumerState.initState]).
  Future<void> initMenu(WidgetRef ref) async {
    _ref = ref;
    await _rebuildMenu();

    // Re-build the menu whenever the active context or context list changes.
    // listenManual is safe to call outside build(); caller owns the subscription.
    _subscriptions.add(
      ref.listenManual<AsyncValue<AssistantContext?>>(
        activeContextProvider,
        (prev, next) => _rebuildMenu(),
      ),
    );
    _subscriptions.add(
      ref.listenManual<AsyncValue<List<AssistantContext>>>(
        contextsProvider,
        (prev, next) => _rebuildMenu(),
      ),
    );
  }

  void dispose() {
    for (final sub in _subscriptions) {
      sub.close();
    }
    _subscriptions.clear();
    trayManager.removeListener(this);
  }

  Future<void> _rebuildMenu() async {
    final ref = _ref;
    if (ref == null) return;

    final activeContext = ref.read(activeContextProvider).value;
    final contexts = ref.read(contextsProvider).value ?? [];

    final activeLabel = activeContext != null
        ? activeContext.name
        : 'No active context';

    // Build "Switch Context" submenu items.
    final switchItems = contexts.map((ctx) {
      final isActive = ctx.id == activeContext?.id;
      return MenuItem(
        key: 'switch_${ctx.id}',
        label: isActive ? '✓ ${ctx.name}' : ctx.name,
      );
    }).toList();

    final menu = Menu(
      items: [
        MenuItem(
          key: 'active_context_label',
          label: activeLabel,
          disabled: true,
        ),
        MenuItem.separator(),
        if (switchItems.isNotEmpty)
          MenuItem.submenu(
            label: 'Switch Context',
            submenu: Menu(items: switchItems),
          ),
        MenuItem(key: 'show_window', label: 'Open'),
        MenuItem.separator(),
        MenuItem(key: 'quit', label: 'Quit'),
      ],
    );
    await trayManager.setContextMenu(menu);
  }

  @override
  void onTrayIconMouseDown() {
    trayManager.popUpContextMenu();
  }

  @override
  void onTrayIconRightMouseDown() {
    trayManager.popUpContextMenu();
  }

  @override
  void onTrayMenuItemClick(MenuItem menuItem) {
    switch (menuItem.key) {
      case 'show_window':
        windowManager.show();
        windowManager.focus();
      case 'quit':
        windowManager.destroy();
      default:
        if (menuItem.key?.startsWith('switch_') == true) {
          final contextId = menuItem.key!.substring('switch_'.length);
          _activateContextById(contextId);
        }
    }
  }

  Future<void> _activateContextById(String id) async {
    final ref = _ref;
    if (ref == null) return;
    final contexts = ref.read(contextsProvider).value ?? [];
    try {
      final ctx = contexts.firstWhere((c) => c.id == id);
      await ref.read(activeContextProvider.notifier).activate(ctx);
    } catch (_) {
      // Context no longer exists — ignore.
    }
  }
}
