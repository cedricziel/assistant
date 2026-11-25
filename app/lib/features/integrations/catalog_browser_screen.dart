import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';
import 'catalog_entry_detail_sheet.dart';
import 'caller_roles.dart';
import 'catalog_entry_tile.dart';
import 'catalog_view_model.dart';
import 'install_integration_sheet.dart';
import 'install_integration_view_model.dart';
import 'integration_status_view.dart';
import 'integration_policy_view_model.dart';

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
          // Left-aligned with the title; extra width becomes right margin.
          final sidePadding = EdgeInsets.fromLTRB(
            AssistantSpacing.lg,
            0,
            (constraints.maxWidth - AssistantSpacing.lg - _maxContentWidth)
                .clamp(AssistantSpacing.lg, double.infinity),
            0,
          );
          return CustomScrollView(
            slivers: [
              const AdaptiveSliverNavBar(title: 'Integrations'),
              SliverPadding(
                padding: sidePadding,
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
              const SliverToBoxAdapter(child: _CustomIntegrationAction()),
              ...switch (state.value) {
                null => [_centered(const CircularProgressIndicator.adaptive())],
                final catalog => _dataSlivers(catalog, sidePadding, notifier),
              },
            ],
          );
        },
      ),
    );
  }

  List<Widget> _dataSlivers(
    CatalogState catalog,
    EdgeInsets sidePadding,
    CatalogViewModel notifier,
  ) {
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
    return IntegrationStatusView(
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
    return IntegrationStatusView(
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

/// "Add custom integration", shown only when the caller may add one.
class _CustomIntegrationAction extends ConsumerWidget {
  const _CustomIntegrationAction();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final policy = ref.watch(integrationPolicyViewModelProvider).value?.policy;
    final roles = ref.watch(callerRolesProvider).value;
    final args = policy == null || roles == null
        ? null
        : installArgsForCustom(policy, roles);
    if (args == null) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: AssistantSpacing.lg),
      child: Align(
        alignment: Alignment.centerLeft,
        child: AdaptiveButton.plain(
          onPressed: () => showInstallIntegrationSheet(context, args),
          child: const Text('Add custom integration'),
        ),
      ),
    );
  }
}
