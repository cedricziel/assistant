## Completed Tasks

- [x] 1. Replace `bottomNavigationBar` with `Scaffold.drawer` + `AppBar(leading: DrawerButton())` in `NavShell` narrow branch (`app/lib/shared/nav_shell.dart`)
- [x] 2. Add primary destinations as `ListTile` widgets in a `ListView`-backed `Drawer`
- [x] 3. Add `Divider` + Settings `ListTile` as a separate section below primary destinations
- [x] 4. Move PWA install option from `_InstallBanner` into the drawer as a conditional `ListTile`
- [x] 5. Delete unused `_InstallBanner` widget
- [x] 6. Each destination `ListTile.onTap` calls `context.go(path)` + `Navigator.of(context).pop()` to close the drawer
- [x] 7. Add `app/test/widget/nav_shell_test.dart` with 13 widget tests (narrow/wide structure, drawer contents, Settings divider, PWA install visibility, close-on-navigation)
- [x] 8. Run `flutter analyze` and `flutter test` — zero issues, all 115 tests pass

## Future Tasks

- [ ] 9. Replace `DrawerHeader` placeholder with a user/server switcher chip connected to `serverProfileProvider`
- [ ] 10. Move Settings + user switcher to `NavigationRail.trailing` on wide layout (separate change)
