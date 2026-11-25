import '../../shared/platform/widgets.dart';

class EmptyChat extends StatelessWidget {
  const EmptyChat({super.key});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.chat_bubble_outline,
            size: 64,
            color: colorScheme.outlineVariant,
          ),
          const SizedBox(height: 16),
          Text(
            'Start a conversation',
            style: TextStyle(
              fontSize: 18,
              color: colorScheme.onSurfaceVariant,
              fontWeight: FontWeight.w500,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Type a message below to begin.',
            style: TextStyle(color: colorScheme.onSurfaceVariant),
          ),
        ],
      ),
    );
  }
}
