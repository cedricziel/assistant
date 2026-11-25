import 'package:material_ui/material_ui.dart';

/// Compact icon button used in the [NavigationRail] trailing slot.
class InstallButton extends StatelessWidget {
  const InstallButton({super.key, required this.onTap});

  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return IconButton.filledTonal(
      icon: const Icon(Icons.install_mobile),
      tooltip: 'Install App',
      onPressed: onTap,
    );
  }
}

/// Slim banner shown above the mobile [NavigationBar].
class InstallBanner extends StatelessWidget {
  const InstallBanner({super.key, required this.onInstall});

  final VoidCallback onInstall;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.primaryContainer,
      child: InkWell(
        onTap: onInstall,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Row(
            children: [
              Icon(Icons.install_mobile, color: colorScheme.onPrimaryContainer),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  'Install App',
                  style: TextStyle(
                    color: colorScheme.onPrimaryContainer,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ),
              Icon(Icons.arrow_forward, color: colorScheme.onPrimaryContainer),
            ],
          ),
        ),
      ),
    );
  }
}

/// Icon button shown in the [NavigationRail] trailing slot on iOS Safari.
/// Tapping it opens a bottom sheet explaining how to add to home screen.
class SafariInstallButton extends StatelessWidget {
  const SafariInstallButton({super.key});

  @override
  Widget build(BuildContext context) {
    return IconButton.filledTonal(
      icon: const Icon(Icons.ios_share),
      tooltip: 'Add to Home Screen',
      onPressed: () => _showSafariInstructions(context),
    );
  }
}

/// Slim banner above the mobile [NavigationBar] on iOS Safari.
class SafariInstallBanner extends StatelessWidget {
  const SafariInstallBanner({super.key});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.primaryContainer,
      child: InkWell(
        onTap: () => _showSafariInstructions(context),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Row(
            children: [
              Icon(Icons.ios_share, color: colorScheme.onPrimaryContainer),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  'Add to Home Screen',
                  style: TextStyle(
                    color: colorScheme.onPrimaryContainer,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ),
              Icon(Icons.arrow_forward, color: colorScheme.onPrimaryContainer),
            ],
          ),
        ),
      ),
    );
  }
}

/// Shows a bottom sheet explaining iOS Safari's manual install steps.
void _showSafariInstructions(BuildContext context) {
  showModalBottomSheet<void>(
    context: context,
    builder: (sheetContext) {
      final colorScheme = Theme.of(sheetContext).colorScheme;
      return SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(24, 24, 24, 32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'Add to Home Screen',
                style: Theme.of(sheetContext).textTheme.titleLarge,
              ),
              const SizedBox(height: 16),
              _SafariStep(
                number: '1',
                icon: Icons.ios_share,
                color: colorScheme.primary,
                text: 'Tap the Share button in the Safari toolbar.',
              ),
              const SizedBox(height: 12),
              _SafariStep(
                number: '2',
                icon: Icons.add_box_outlined,
                color: colorScheme.primary,
                text: 'Scroll down and tap "Add to Home Screen".',
              ),
              const SizedBox(height: 12),
              _SafariStep(
                number: '3',
                icon: Icons.check_circle_outline,
                color: colorScheme.primary,
                text: 'Tap "Add" to confirm.',
              ),
            ],
          ),
        ),
      );
    },
  );
}

class _SafariStep extends StatelessWidget {
  const _SafariStep({
    required this.number,
    required this.icon,
    required this.color,
    required this.text,
  });

  final String number;
  final IconData icon;
  final Color color;
  final String text;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        CircleAvatar(
          radius: 14,
          backgroundColor: color,
          child: Text(
            number,
            style: TextStyle(
              color: Theme.of(context).colorScheme.onPrimary,
              fontWeight: FontWeight.bold,
              fontSize: 13,
            ),
          ),
        ),
        const SizedBox(width: 12),
        Icon(icon, color: color),
        const SizedBox(width: 8),
        Expanded(child: Text(text)),
      ],
    );
  }
}
