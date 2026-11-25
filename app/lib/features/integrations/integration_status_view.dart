import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';

/// Centered icon, optional title, message and action for empty and error
/// states.
class IntegrationStatusView extends StatelessWidget {
  const IntegrationStatusView({
    super.key,
    required this.icon,
    required this.iconColor,
    required this.message,
    this.title,
    this.action,
  });

  /// A failed load, with a retry action.
  IntegrationStatusView.error({
    super.key,
    required BuildContext context,
    required this.message,
    required VoidCallback onRetry,
  }) : icon = Icons.error_outline,
       iconColor = Theme.of(context).colorScheme.error,
       title = null,
       action = AdaptiveButton.filled(
         onPressed: onRetry,
         child: const Text('Retry'),
       );

  final IconData icon;
  final Color iconColor;
  final String? title;
  final String message;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final muted = Theme.of(context).colorScheme.onSurfaceVariant;
    return Padding(
      padding: const EdgeInsets.all(AssistantSpacing.xxl),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 48, color: iconColor),
          const SizedBox(height: AssistantSpacing.lg),
          if (title != null) ...[
            Text(
              title!,
              style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: AssistantSpacing.sm),
          ],
          Text(
            message,
            textAlign: TextAlign.center,
            style: title == null ? null : TextStyle(color: muted),
          ),
          if (action != null) ...[
            const SizedBox(height: AssistantSpacing.lg),
            action!,
          ],
        ],
      ),
    );
  }
}
