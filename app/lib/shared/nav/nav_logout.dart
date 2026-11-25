import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import '../../router/app_router.dart';
import '../auth_actions.dart';

/// Logs out and returns to the login screen.
Future<void> logoutAndShowLogin(BuildContext context) async {
  await performLogout(ProviderScope.containerOf(context, listen: false));
  if (context.mounted) {
    context.go(AppRoutes.login);
  }
}
