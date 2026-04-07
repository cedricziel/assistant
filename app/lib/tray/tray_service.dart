import 'package:tray_manager/tray_manager.dart';
import 'package:window_manager/window_manager.dart';

/// Initializes the macOS menu bar tray icon and wires up context-menu actions.
///
/// Call [init] once on startup (macOS only). The tray icon shows an "Open"
/// item that shows/focuses the main window, and a "Quit" item that exits.
class TrayService with TrayListener {
  static final TrayService _instance = TrayService._();
  factory TrayService() => _instance;
  TrayService._();

  Future<void> init() async {
    trayManager.addListener(this);

    await trayManager.setIcon('assets/tray_icon.png');

    final menu = Menu(
      items: [
        MenuItem(
          key: 'show_window',
          label: 'Open',
        ),
        MenuItem.separator(),
        MenuItem(
          key: 'quit',
          label: 'Quit',
        ),
      ],
    );
    await trayManager.setContextMenu(menu);
  }

  void dispose() {
    trayManager.removeListener(this);
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
    }
  }
}
