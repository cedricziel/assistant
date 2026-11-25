import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';
import 'catalog_entry_detail_sheet.dart';
import 'catalog_entry_tile.dart';
import 'catalog_view_model.dart';

/// Widest the browser grows; wider windows get side margins.
const _maxContentWidth = 840.0;

/// Browse and search the integrations catalog.
class CatalogBrowserScreen extends ConsumerStatefulWidget {
  const CatalogBrowserScreen({super.key});

  @override
  ConsumerState<CatalogBrowserScreen> createState() =>
      _CatalogBrowserScreenState();
}

class _CatalogBrowserScreenState extends ConsumerState<CatalogBrowserScreen> {
  final _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _clearFilters() {
    _searchController.clear();
    ref.read(catalogViewModelProvider.notifier).clearFilters();
  }

  @override
  Widget build(BuildContext context) {
    final notifier = ref.read(catalogViewModelProvider.notifier);
    final state = ref.watch(catalogViewModelProvider);

    return AdaptiveScaffold(
      body: LayoutBuilder(
        builder: (context, constraints) {
          final gutter = ((constraints.maxWidth - _maxContentWidth) / 2).clamp(
            AssistantSpacing.lg,
            double.infinity,
          );
          return CustomScrollView(
            slivers: [
              const AdaptiveSliverNavBar(title: 'Integrations'),
              SliverPadding(
                padding: EdgeInsets.symmetric(horizontal: gutter),
                sliver: SliverToBoxAdapter(
                  child: AdaptiveTextField(
                    controller: _searchController,
                    placeholder: 'Search integrations',
                    prefix: const Icon(Icons.search),
                    textInputAction: TextInputAction.search,
                    onChanged: notifier.setQuery,
                  ),
                ),
              ),
              ...switch (state.value) {
                null => [_centered(const CircularProgressIndicator.adaptive())],
                final catalog => _dataSlivers(catalog, gutter, notifier),
              },
            ],
          );
        },
      ),
    );
  }

  List<Widget> _dataSlivers(
    CatalogState catalog,
    double gutter,
    CatalogViewModel notifier,
  ) {
    final sidePadding = EdgeInsets.symmetric(horizontal: gutter);
    final chips = SliverPadding(
      padding: sidePadding.copyWith(top: AssistantSpacing.md),
      sliver: SliverToBoxAdapter(
        child: _CategoryChips(
          categories: catalog.categories,
          selected: catalog.category,
          onSelected: notifier.selectCategory,
        ),
      ),
    );

    if (catalog.error != null) {
      return [
        chips,
        _centered(_ErrorView(error: catalog.error!, onRetry: notifier.retry)),
      ];
    }
    if (catalog.items.isEmpty) {
      return [
        chips,
        _centered(
          catalog.isLoading
              ? const CircularProgressIndicator.adaptive()
              : _EmptyView(
                  filtered:
                      catalog.query.isNotEmpty || catalog.category != null,
                  onClearFilters: _clearFilters,
                ),
        ),
      ];
    }

    return [
      chips,
      SliverPadding(
        padding: sidePadding.copyWith(top: AssistantSpacing.md),
        sliver: SliverOpacity(
          opacity: catalog.isLoading ? 0.5 : 1,
          sliver: SliverList.separated(
            itemCount: catalog.items.length,
            separatorBuilder: (_, _) =>
                const SizedBox(height: AssistantSpacing.sm),
            itemBuilder: (context, index) {
              final entry = catalog.items[index];
              return CatalogEntryTile(
                key: ValueKey(entry.id),
                entry: entry,
                onTap: () => showCatalogEntryDetail(context, entry),
              );
            },
          ),
        ),
      ),
      if (catalog.nextCursor != null)
        SliverPadding(
          padding: sidePadding.copyWith(
            top: AssistantSpacing.md,
            bottom: AssistantSpacing.xl,
          ),
          sliver: SliverToBoxAdapter(
            child: _LoadMore(
              isLoading: catalog.isLoadingMore,
              failed: catalog.loadMoreFailed,
              onPressed: notifier.loadMore,
            ),
          ),
        )
      else
        const SliverToBoxAdapter(child: SizedBox(height: AssistantSpacing.xl)),
    ];
  }

  Widget _centered(Widget child) =>
      SliverFillRemaining(hasScrollBody: false, child: Center(child: child));
}

class _CategoryChips extends StatelessWidget {
  const _CategoryChips({
    required this.categories,
    required this.selected,
    required this.onSelected,
  });

  final List<String> categories;
  final String? selected;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    if (categories.isEmpty) return const SizedBox.shrink();
    return Wrap(
      spacing: AssistantSpacing.sm,
      runSpacing: AssistantSpacing.xs,
      children: [
        for (final category in categories)
          ChoiceChip(
            label: Text(category),
            selected: category == selected,
            onSelected: (_) => onSelected(category),
          ),
      ],
    );
  }
}

class _LoadMore extends StatelessWidget {
  const _LoadMore({
    required this.isLoading,
    required this.failed,
    required this.onPressed,
  });

  final bool isLoading;
  final bool failed;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    if (isLoading) {
      return const Center(child: CircularProgressIndicator.adaptive());
    }
    return Center(
      child: AdaptiveButton.plain(
        onPressed: onPressed,
        child: Text(failed ? 'Could not load more. Try again' : 'Load more'),
      ),
    );
  }
}

class _EmptyView extends StatelessWidget {
  const _EmptyView({required this.filtered, required this.onClearFilters});

  final bool filtered;
  final VoidCallback onClearFilters;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return _StatusView(
      icon: Icons.hub_outlined,
      iconColor: colors.onSurfaceVariant,
      title: filtered
          ? 'No integrations match your search'
          : 'No integrations available',
      message: filtered
          ? 'Try a different search term or category.'
          : 'The catalog is empty or has not been synced yet. '
                'Ask an administrator to check the catalog source.',
      action: filtered
          ? AdaptiveButton.plain(
              onPressed: onClearFilters,
              child: const Text('Clear filters'),
            )
          : null,
    );
  }
}

class _ErrorView extends StatelessWidget {
  const _ErrorView({required this.error, required this.onRetry});

  final String error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return _StatusView(
      icon: Icons.error_outline,
      iconColor: Theme.of(context).colorScheme.error,
      message: 'Failed to load the catalog: $error',
      action: AdaptiveButton.filled(
        onPressed: onRetry,
        child: const Text('Retry'),
      ),
    );
  }
}

/// Centered icon, optional title, message and action for empty and error
/// states.
class _StatusView extends StatelessWidget {
  const _StatusView({
    required this.icon,
    required this.iconColor,
    required this.message,
    this.title,
    this.action,
  });

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
