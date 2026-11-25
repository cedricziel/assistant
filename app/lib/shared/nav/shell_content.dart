import 'package:material_ui/material_ui.dart';

import '../../features/notifications/agent_event_listener.dart';
import '../../features/updater/update_banner.dart';
import 'offline_banner.dart';

/// The routed page wrapped with the shell-level listeners and banners shared
/// by every navigation layout.
class ShellContent extends StatelessWidget {
  const ShellContent({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return AgentEventListener(
      child: OfflineBanner(child: UpdateBannerWrapper(child: child)),
    );
  }
}
