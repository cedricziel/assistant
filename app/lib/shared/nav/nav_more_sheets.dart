import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import '../../router/app_router.dart';
import 'nav_destinations.dart';
import 'nav_logout.dart';

/// Shows the "More" bottom sheet listing all overflow destinations.
/// Shows the "More" bottom sheet listing all overflow destinations.
void showMaterialMoreSheet(BuildContext context) {
  showModalBottomSheet<void>(
    context: context,
    builder: (sheetContext) {
      return Semantics(
        label: 'More destinations',
        child: SafeArea(
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
                  child: Text(
                    'More',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                // Space switcher at the top of the More sheet.
                ListTile(
                  leading: const Icon(Icons.workspaces_outlined),
                  title: const Text('Switch space'),
                  onTap: () {
                    Navigator.of(sheetContext).pop();
                    context.go(AppRoutes.spaceSelector);
                  },
                ),
                // Contexts switcher.
                ListTile(
                  leading: Icon(contextsDestination.icon),
                  title: Text(contextsDestination.label),
                  onTap: () {
                    Navigator.of(sheetContext).pop();
                    context.go(contextsDestination.path);
                  },
                ),
                ...overflowDestinations.map(
                  (d) => ListTile(
                    leading: Icon(d.icon),
                    title: Text(d.label),
                    onTap: () {
                      Navigator.of(sheetContext).pop();
                      context.go(d.path);
                    },
                  ),
                ),
                // Settings is separated from the rest of the overflow items.
                const Divider(height: 1),
                ListTile(
                  leading: Icon(settingsDestination.icon),
                  title: Text(settingsDestination.label),
                  onTap: () {
                    Navigator.of(sheetContext).pop();
                    context.go(settingsDestination.path);
                  },
                ),
                ListTile(
                  key: const Key('material-more-logout'),
                  leading: const Icon(Icons.logout),
                  title: const Text('Log out'),
                  onTap: () async {
                    Navigator.of(sheetContext).pop();
                    await logoutAndShowLogin(context);
                  },
                ),
                const SizedBox(height: 8),
              ],
            ),
          ),
        ),
      );
    },
  );
}

/// Shows a Cupertino action sheet listing all overflow destinations.
/// Shows a Cupertino action sheet listing all overflow destinations.
void showCupertinoMoreSheet(BuildContext context) {
  showCupertinoModalPopup<void>(
    context: context,
    builder: (popupContext) {
      return CupertinoActionSheet(
        title: const Text('More'),
        actions: [
          CupertinoActionSheetAction(
            onPressed: () {
              Navigator.of(popupContext).pop();
              context.go(AppRoutes.spaceSelector);
            },
            child: const Text('Switch space'),
          ),
          ...overflowDestinations.map(
            (d) => CupertinoActionSheetAction(
              onPressed: () {
                Navigator.of(popupContext).pop();
                context.go(d.path);
              },
              child: Text(d.label),
            ),
          ),
          CupertinoActionSheetAction(
            onPressed: () {
              Navigator.of(popupContext).pop();
              context.go(settingsDestination.path);
            },
            child: Text(settingsDestination.label),
          ),
          CupertinoActionSheetAction(
            key: const Key('cupertino-more-logout'),
            isDestructiveAction: true,
            onPressed: () async {
              Navigator.of(popupContext).pop();
              await logoutAndShowLogin(context);
            },
            child: const Text('Log out'),
          ),
        ],
        cancelButton: CupertinoActionSheetAction(
          isDefaultAction: true,
          onPressed: () => Navigator.of(popupContext).pop(),
          child: const Text('Cancel'),
        ),
      );
    },
  );
}
