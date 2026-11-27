import 'package:material_ui/material_ui.dart';

/// A model the user can regenerate a reply with.
class RegenerateModel {
  const RegenerateModel({required this.name, this.isDefault = false});

  final String name;

  /// Marks the model the server uses when none is chosen.
  final bool isDefault;
}

/// "Regenerate with…" menu: one entry per model, the default marked.
///
/// Renders nothing with fewer than two [models], since there is nothing to
/// choose between. Picking an entry calls [onSelected] with the model name.
class RegenerateMenuButton extends StatelessWidget {
  const RegenerateMenuButton({
    super.key,
    required this.models,
    required this.onSelected,
  });

  static const _label = 'Regenerate with…';

  final List<RegenerateModel> models;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    if (models.length < 2) return const SizedBox.shrink();

    final scheme = Theme.of(context).colorScheme;
    final muted = scheme.onSurfaceVariant;

    return PopupMenuButton<String>(
      tooltip: _label,
      padding: EdgeInsets.zero,
      onSelected: onSelected,
      itemBuilder: (context) => [
        for (final model in models)
          PopupMenuItem<String>(
            value: model.name,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(model.name),
                if (model.isDefault) ...[
                  const SizedBox(width: 8),
                  Text('default', style: TextStyle(fontSize: 12, color: muted)),
                ],
              ],
            ),
          ),
      ],
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.auto_awesome_outlined, size: 14, color: muted),
            const SizedBox(width: 4),
            Text(_label, style: TextStyle(fontSize: 12, color: muted)),
            Icon(Icons.arrow_drop_down, size: 14, color: muted),
          ],
        ),
      ),
    );
  }
}
