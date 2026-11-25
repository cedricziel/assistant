import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../api/connectivity_provider.dart';
import '../platform/platform.dart';

/// Persistent banner shown when the device has no network connectivity.
///
/// Skipped on web — browsers provide their own offline indicators.
/// Uses a Cupertino-style slim bar on iOS, MaterialBanner elsewhere.
class OfflineBanner extends ConsumerWidget {
  const OfflineBanner({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (kIsWeb) return child;

    final isOnline = ref.watch(isOnlineProvider);
    if (isOnline) return child;

    return Column(
      children: [
        if (isAppleTouch)
          Builder(
            builder: (context) {
              final fgColor = Theme.of(context).colorScheme.onInverseSurface;
              return Container(
                width: double.infinity,
                color: CupertinoColors.systemOrange.resolveFrom(context),
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Icon(CupertinoIcons.wifi_slash, size: 14, color: fgColor),
                    const SizedBox(width: 6),
                    Text(
                      'No Connection',
                      style: TextStyle(
                        color: fgColor,
                        fontSize: 13,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                  ],
                ),
              );
            },
          )
        else
          MaterialBanner(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            leading: const Icon(Icons.wifi_off),
            content: const Text('No internet connection'),
            actions: const [SizedBox.shrink()],
          ),
        Expanded(child: child),
      ],
    );
  }
}
