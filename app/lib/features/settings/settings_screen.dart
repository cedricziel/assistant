import 'package:flutter/services.dart' show HapticFeedback;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../router/app_router.dart';
import '../../shared/platform/widgets.dart';
import '../notifications/notification_preferences.dart';
import '../notifications/notification_service.dart';

/// Application settings screen.
class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final prefsAsync = ref.watch(notificationPreferencesProvider);

    return AdaptiveScaffold(
      body: CustomScrollView(
        slivers: [
          const AdaptiveSliverNavBar(title: 'Settings'),
          SliverToBoxAdapter(
            child: AdaptiveListSection(
              header: const Text('Account'),
              children: [
                AdaptiveListTile(
                  leading: const AdaptiveIcon(
                    cupertino: CupertinoIcons.person,
                    material: Icons.person_outline,
                  ),
                  title: const Text('Account'),
                  subtitle: const Text('Name, email, and password'),
                  trailing: const AdaptiveIcon(
                    cupertino: CupertinoIcons.chevron_forward,
                    material: Icons.chevron_right,
                  ),
                  onTap: () => context.push(AppRoutes.settingsAccount),
                ),
              ],
            ),
          ),
          SliverToBoxAdapter(
            child: prefsAsync.when(
              loading: () =>
                  const Center(child: CircularProgressIndicator.adaptive()),
              error: (e, _) => AdaptiveListTile(
                leading: const Icon(Icons.error_outline),
                title: Text('Failed to load preferences: $e'),
              ),
              data: (prefs) => AdaptiveListSection(
                header: const Text('Notifications'),
                children: [
                  AdaptiveSwitchTile(
                    leading: const AdaptiveIcon(
                      cupertino: CupertinoIcons.chat_bubble,
                      material: Icons.chat_bubble_outline,
                    ),
                    title: const Text('New chat messages'),
                    subtitle: const Text(
                      'Notify when the assistant sends a new message',
                    ),
                    value: prefs.notifyChatMessages,
                    onChanged: (value) async {
                      if (isAppleTouch) HapticFeedback.lightImpact();
                      await prefs.setNotifyChatMessages(value);
                      ref.invalidate(notificationPreferencesProvider);
                    },
                  ),
                  AdaptiveSwitchTile(
                    leading: const AdaptiveIcon(
                      cupertino: CupertinoIcons.square_grid_2x2,
                      material: Icons.extension_outlined,
                    ),
                    title: const Text('Skill completions'),
                    subtitle: const Text(
                      'Notify when a skill run succeeds or fails',
                    ),
                    value: prefs.notifySkillComplete,
                    onChanged: (value) async {
                      if (isAppleTouch) HapticFeedback.lightImpact();
                      await prefs.setNotifySkillComplete(value);
                      ref.invalidate(notificationPreferencesProvider);
                    },
                  ),
                  AdaptiveSwitchTile(
                    leading: const AdaptiveIcon(
                      cupertino: CupertinoIcons.exclamationmark_triangle,
                      material: Icons.warning_amber_outlined,
                    ),
                    title: const Text('Agent errors'),
                    subtitle: const Text('Notify on critical assistant errors'),
                    value: prefs.notifyAgentErrors,
                    onChanged: (value) async {
                      if (isAppleTouch) HapticFeedback.lightImpact();
                      await prefs.setNotifyAgentErrors(value);
                      ref.invalidate(notificationPreferencesProvider);
                    },
                  ),
                  AdaptiveListTile(
                    leading: const AdaptiveIcon(
                      cupertino: CupertinoIcons.bell,
                      material: Icons.notifications_outlined,
                    ),
                    title: const Text('Request permission'),
                    trailing: const AdaptiveIcon(
                      cupertino: CupertinoIcons.chevron_forward,
                      material: Icons.chevron_right,
                    ),
                    onTap: () async {
                      final ns = ref.read(notificationServiceProvider);
                      await ns.initialize();
                      await ns.requestPermission();
                    },
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
