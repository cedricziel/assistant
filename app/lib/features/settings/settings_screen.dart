import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/platform.dart';
import '../notifications/notification_preferences.dart';
import '../notifications/notification_service.dart';

/// Application settings screen.
class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final prefsAsync = ref.watch(notificationPreferencesProvider);

    final bodyChildren = [
      // -- Notifications -----------------------------------------------
      const _SectionHeader('Notifications'),
      prefsAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => ListTile(
          leading: const Icon(Icons.error_outline),
          title: Text('Failed to load preferences: $e'),
        ),
        data: (prefs) => Column(
          children: [
            SwitchListTile(
              secondary: const Icon(Icons.chat_bubble_outline),
              title: const Text('New chat messages'),
              subtitle: const Text(
                'Notify when the assistant sends a new message',
              ),
              value: prefs.notifyChatMessages,
              onChanged: (value) async {
                await prefs.setNotifyChatMessages(value);
                // Refresh provider so UI stays in sync.
                ref.invalidate(notificationPreferencesProvider);
              },
            ),
            SwitchListTile(
              secondary: const Icon(Icons.extension_outlined),
              title: const Text('Skill completions'),
              subtitle: const Text('Notify when a skill run succeeds or fails'),
              value: prefs.notifySkillComplete,
              onChanged: (value) async {
                await prefs.setNotifySkillComplete(value);
                ref.invalidate(notificationPreferencesProvider);
              },
            ),
            SwitchListTile(
              secondary: const Icon(Icons.warning_amber_outlined),
              title: const Text('Agent errors'),
              subtitle: const Text('Notify on critical assistant errors'),
              value: prefs.notifyAgentErrors,
              onChanged: (value) async {
                await prefs.setNotifyAgentErrors(value);
                ref.invalidate(notificationPreferencesProvider);
              },
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: OutlinedButton.icon(
                icon: const Icon(Icons.notifications_outlined),
                label: const Text('Request notification permission'),
                onPressed: () async {
                  final ns = ref.read(notificationServiceProvider);
                  await ns.initialize();
                  await ns.requestPermission();
                },
              ),
            ),
          ],
        ),
      ),
    ];

    if (isAppleTouch) {
      return Scaffold(
        body: CustomScrollView(
          slivers: [
            const CupertinoSliverNavigationBar(largeTitle: Text('Settings')),
            SliverList(delegate: SliverChildListDelegate(bodyChildren)),
          ],
        ),
      );
    }

    return Scaffold(
      appBar: AppBar(title: const Text('Settings')),
      body: ListView(children: bodyChildren),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader(this.title);

  final String title;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 20, 16, 4),
      child: Text(
        title,
        style: Theme.of(context).textTheme.labelLarge?.copyWith(
          color: Theme.of(context).colorScheme.primary,
        ),
      ),
    );
  }
}
