import 'dart:async';
import 'dart:ui' show Size;

import 'package:flutter_riverpod/flutter_riverpod.dart'
    show WidgetRef, AsyncValue, ProviderSubscription;
import 'package:tray_manager/tray_manager.dart';
import 'package:window_manager/window_manager.dart';

import '../features/contexts/models/context_model.dart';
import '../features/contexts/providers/context_providers.dart';

/// One attached context menu and every native wrapper that has to outlive it.
///
/// nativeapi frees a native handle once its Dart wrapper is collected, and the
/// attached menu still points at those handles, so the service has to keep the
/// menu, its items and its submenus reachable until it swaps them out.
class _TrayMenu {
  _TrayMenu(this.menu);

  final Menu menu;
  final List<MenuItem> items = [];
  final List<Menu> submenus = [];

  void dispose() {
    for (final item in items) {
      item.dispose();
    }
    items.clear();
    for (final submenu in submenus) {
      submenu.dispose();
    }
    submenus.clear();
    menu.dispose();
  }
}

/// Initializes the macOS menu bar tray icon and wires up context-menu actions.
///
/// Call [init] once on startup (macOS only).  Holds a [Ref] so it can read
/// [contextsProvider] and [activeContextProvider] to populate the menu.
class TrayService {
  static final TrayService _instance = TrayService._();
  factory TrayService() => _instance;
  TrayService._();

  TrayIcon? _trayIcon;
  Image? _icon;
  ListenerId? _trayListenerId;
  _TrayMenu? _menu;

  WidgetRef? _ref;
  final List<ProviderSubscription<dynamic>> _subscriptions = [];

  /// Sets up the tray icon.  Call this before [runApp] (no [Ref] required).
  Future<void> initIcon() async {
    if (_trayIcon != null) return;

    final trayIcon = TrayIcon.create();
    if (trayIcon == null) return;

    final icon = ImageAsset.fromAsset('assets/tray_icon.png');
    if (icon != null) {
      trayIcon.icon = icon;
      _icon = icon;
    }
    trayIcon.isIconTemplate = true;
    trayIcon.iconSize = const Size.square(18);
    trayIcon.setVisible(true);

    // A menu bar icon opens its menu on click, with either button.
    _trayListenerId = trayIcon.addListener((event) {
      if (event is TrayIconClickedEvent || event is TrayIconRightClickedEvent) {
        trayIcon.openContextMenu();
      }
    });

    _trayIcon = trayIcon;

    // Show a static menu until [initMenu] is called with a Ref.
    _attachMenu(_buildMenu(null, const []));
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

    final trayIcon = _trayIcon;
    final listenerId = _trayListenerId;
    if (trayIcon != null && listenerId != null) {
      trayIcon.removeListener(listenerId);
    }
    _trayListenerId = null;

    _trayMenu = null;
    _icon?.dispose();
    _icon = null;
    trayIcon?.dispose();
    _trayIcon = null;
  }

  set _trayMenu(_TrayMenu? menu) {
    final previous = _menu;
    _menu = menu;
    // A click listener may be running inside a native callback of the menu it
    // is replacing, so let it return before the old handles are freed.
    if (previous != null) {
      Timer.run(previous.dispose);
    }
  }

  Future<void> _rebuildMenu() async {
    final ref = _ref;
    if (ref == null) return;

    final activeContext = ref.read(activeContextProvider).value;
    final contexts = ref.read(contextsProvider).value ?? [];

    _attachMenu(_buildMenu(activeContext, contexts));
  }

  void _attachMenu(_TrayMenu? menu) {
    final trayIcon = _trayIcon;
    if (trayIcon == null || menu == null) return;
    trayIcon.setContextMenu(menu.menu);
    _trayMenu = menu;
  }

  _TrayMenu? _buildMenu(
    AssistantContext? activeContext,
    List<AssistantContext> contexts,
  ) {
    final menu = Menu.create();
    if (menu == null) return null;
    final result = _TrayMenu(menu);

    final activeLabel = activeContext != null
        ? activeContext.name
        : 'No active context';
    final labelItem = MenuItem.createWithLabelAndType(
      activeLabel,
      MenuItemType.normal,
    );
    if (labelItem != null) {
      labelItem.isEnabled = false;
      result.items.add(labelItem);
      menu.addItem(labelItem);
    }

    menu.addSeparator();

    // Build "Switch Context" submenu items.
    final submenu = contexts.isEmpty ? null : Menu.create();
    final submenuItem = contexts.isEmpty
        ? null
        : MenuItem.createWithLabelAndType(
            'Switch Context',
            MenuItemType.submenu,
          );
    if (submenu != null && submenuItem != null) {
      result.submenus.add(submenu);
      for (final ctx in contexts) {
        final isActive = ctx.id == activeContext?.id;
        final item = MenuItem.createWithLabelAndType(
          isActive ? '✓ ${ctx.name}' : ctx.name,
          MenuItemType.normal,
        );
        if (item == null) continue;
        result.items.add(item);
        submenu.addItem(item);
        item.addListener((event) {
          if (event is MenuItemClickedEvent) {
            _activateContextById(ctx.id);
          }
        });
      }
      submenuItem.submenu = submenu;
      result.items.add(submenuItem);
      menu.addItem(submenuItem);
    }

    final openItem = MenuItem.createWithLabelAndType(
      'Open',
      MenuItemType.normal,
    );
    if (openItem != null) {
      result.items.add(openItem);
      menu.addItem(openItem);
      openItem.addListener((event) {
        if (event is MenuItemClickedEvent) {
          windowManager.show();
          windowManager.focus();
        }
      });
    }

    menu.addSeparator();

    final quitItem = MenuItem.createWithLabelAndType(
      'Quit',
      MenuItemType.normal,
    );
    if (quitItem != null) {
      result.items.add(quitItem);
      menu.addItem(quitItem);
      quitItem.addListener((event) {
        if (event is MenuItemClickedEvent) {
          windowManager.destroy();
        }
      });
    }

    return result;
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
