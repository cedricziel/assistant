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
              ...state.when(
                loading: () => [_centered(const _Loading())],
                error: (err, _) => [
                  _centered(_ErrorView(error: '$err', onRetry: notifier.retry)),
                ],
                data: (catalog) => _dataSlivers(catalog, gutter, notifier),
              ),
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
              ? const _Loading()
              : _EmptyView(
                  filtered:
                      catalog.query.isNotEmpty || catalog.category != null,
                  onClearFilters: _clearFilters,
                ),
        ),
      ];
    }

    final items = catalog.items;
    return [
      chips,
      SliverPadding(
        padding: sidePadding.copyWith(top: AssistantSpacing.md),
        sliver: SliverOpacity(
          opacity: catalog.isLoading ? 0.5 : 1,
          sliver: SliverList.separated(
            itemCount: items.length,
            separatorBuilder: (_, _) =>
                const SizedBox(height: AssistantSpacing.sm),
            itemBuilder: (context, index) {
              final entry = items[index];
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

class _Loading extends StatelessWidget {
  const _Loading();

  @override
  Widget build(BuildContext context) =>
      const CircularProgressIndicator.adaptive();
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
    final muted = Theme.of(context).colorScheme.onSurfaceVariant;
    return Padding(
      padding: const EdgeInsets.all(AssistantSpacing.xxl),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.hub_outlined, size: 48, color: muted),
          const SizedBox(height: AssistantSpacing.lg),
          Text(
            filtered
                ? 'No integrations match your search'
                : 'No integrations available',
            style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: AssistantSpacing.sm),
          Text(
            filtered
                ? 'Try a different search term or category.'
                : 'The catalog is empty or has not been synced yet. '
                      'Ask an administrator to check the catalog source.',
            textAlign: TextAlign.center,
            style: TextStyle(color: muted),
          ),
          if (filtered) ...[
            const SizedBox(height: AssistantSpacing.lg),
            AdaptiveButton.plain(
              onPressed: onClearFilters,
              child: const Text('Clear filters'),
            ),
          ],
        ],
      ),
    );
  }
}

class _ErrorView extends StatelessWidget {
  const _ErrorView({required this.error, required this.onRetry});

  final String error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.all(AssistantSpacing.xxl),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.error_outline, size: 48, color: colors.error),
          const SizedBox(height: AssistantSpacing.lg),
          Text(
            'Failed to load the catalog: $error',
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: AssistantSpacing.lg),
          AdaptiveButton.filled(onPressed: onRetry, child: const Text('Retry')),
        ],
      ),
    );
  }
}
