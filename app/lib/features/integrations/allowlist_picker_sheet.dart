import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';
import 'catalog_view_model.dart';

/// Lets the user tick catalog entries; pops with the chosen ids.
///
/// Entries in [initial] stay ticked even when the current search hides them.
Future<List<String>?> showAllowlistPicker(
  BuildContext context,
  List<String> initial,
) {
  return showModalBottomSheet<List<String>>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    showDragHandle: true,
    constraints: const BoxConstraints(maxWidth: 640),
    builder: (_) => AllowlistPickerSheet(initial: initial),
  );
}

class AllowlistPickerSheet extends ConsumerStatefulWidget {
  const AllowlistPickerSheet({super.key, required this.initial});

  final List<String> initial;

  @override
  ConsumerState<AllowlistPickerSheet> createState() =>
      _AllowlistPickerSheetState();
}

class _AllowlistPickerSheetState extends ConsumerState<AllowlistPickerSheet> {
  late final Set<String> _selected = {...widget.initial};

  @override
  Widget build(BuildContext context) {
    final catalog = ref.watch(catalogViewModelProvider);
    final vm = ref.read(catalogViewModelProvider.notifier);
    final theme = Theme.of(context);

    return Padding(
      padding: EdgeInsets.fromLTRB(
        AssistantSpacing.xl,
        0,
        AssistantSpacing.xl,
        AssistantSpacing.xl + MediaQuery.viewInsetsOf(context).bottom,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text('Allowed catalog entries', style: theme.textTheme.titleLarge),
          const SizedBox(height: AssistantSpacing.md),
          AdaptiveTextField(
            placeholder: 'Search integrations',
            prefix: const Icon(Icons.search),
            onChanged: vm.setQuery,
          ),
          const SizedBox(height: AssistantSpacing.sm),
          Flexible(
            child: switch (catalog) {
              AsyncData(:final value) when value.error != null => Padding(
                padding: const EdgeInsets.all(AssistantSpacing.lg),
                child: Text('Failed to load the catalog: ${value.error}'),
              ),
              AsyncData(:final value) when value.items.isEmpty => const Padding(
                padding: EdgeInsets.all(AssistantSpacing.lg),
                child: Text('No catalog entries match.'),
              ),
              AsyncData(:final value) => ListView(
                shrinkWrap: true,
                children: [for (final entry in value.items) _tile(entry)],
              ),
              _ => const Padding(
                padding: EdgeInsets.all(AssistantSpacing.lg),
                child: Center(child: CircularProgressIndicator.adaptive()),
              ),
            },
          ),
          const SizedBox(height: AssistantSpacing.md),
          Align(
            alignment: Alignment.centerRight,
            child: AdaptiveButton.filled(
              onPressed: () => Navigator.of(context).pop(_selected.toList()),
              child: const Text('Done'),
            ),
          ),
        ],
      ),
    );
  }

  Widget _tile(CatalogEntrySummary entry) {
    return CheckboxListTile(
      value: _selected.contains(entry.id),
      title: Text(entry.name),
      subtitle: Text(entry.domain),
      onChanged: (on) => setState(() {
        if (on ?? false) {
          _selected.add(entry.id);
        } else {
          _selected.remove(entry.id);
        }
      }),
    );
  }
}
